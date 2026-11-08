use gltf::animation::util::ReadOutputs;
use macroquad::prelude::*;
use std::env;

use super::ClipSelectionMode;

/*
    this function is to hard lock specific gltf files to known-good animation clip indexes when a file has multiple clips.
    before implementing this we had a lot of animations where auto selection could pick an idle/t-pose/incorrect clip due to generic clip names,
    so we now keep a deterministic per-file override table based on debug logs.
    these values come from the printed candidate + selected clip output during loading, so there is no guesswork and no extra metadata needed.
    if a source gltf changes clip ordering, update the index here for that file stem.
*/
fn animation_override_index_for_stem(file_stem: &str) -> Option<usize> {
    match file_stem {
        // values below found from log messages
        // each number represents the animation index in the gltf file that matches the intended animation for that file
        "crouch" => Some(8),
        "crouch_aim" => Some(8),
        "crouch_back" => Some(8),
        "crouch_death" => Some(0),
        "crouch_forward" => Some(8),
        "crouch_left" => Some(8),
        "crouch_right" => Some(8),
        "jump" => Some(3),
        "prone_death" => Some(0),
        "prone_forward" => Some(0),
        "prone_idle" => Some(6),
        "prone_turn_right" => Some(0),
        "rifle_aim_idle" => Some(0),
        "rifle_idle" => Some(8),
        "slide" => Some(3),
        "sprint_forward" => Some(0),
        "sprint_left" => Some(0),
        "sprint_right" => Some(0),
        "stand_death" => Some(2),
        "walk_back" => Some(2),
        "walk_forward" => Some(0),
        "walk_left" => Some(0),
        "walk_right" => Some(0),
        "fps_ak-74m_animations" => Some(1), // Rig|AK_Idle
        _ => None,
    }
}

/// Pick the animation clip to play from `doc`: the forced index, the
/// per-file override, or the best candidate by name and `mode`.
pub(super) fn select_clip<'a>(
    path: &str,
    doc: &'a gltf::Document,
    buffers: &[gltf::buffer::Data],
    mode: ClipSelectionMode,
    forced_clip_index: Option<usize>,
) -> Result<gltf::Animation<'a>, String> {
    let file_stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let stem_tokens: Vec<String> = file_stem
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    let override_anim_index = animation_override_index_for_stem(&file_stem);
    let strict_override_mode = env::var("ANIM_REQUIRE_OVERRIDE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let mut selected_anim = None;
    let mut best_name_score = -1i32;
    let mut best_metric = f32::NEG_INFINITY;
    let mut best_duration = f32::NEG_INFINITY;
    let mut multi_clip_count = 0usize;
    let mut override_found = false;
    for anim in doc.animations() {
        if let Some(forced) = forced_clip_index {
            if anim.index() != forced {
                continue;
            }
        }
        multi_clip_count += 1;
        let mut has_channels = false;
        let mut first_t = f32::INFINITY;
        let mut last_t = f32::NEG_INFINITY;
        let mut motion_score = 0.0f32;
        for channel in anim.channels() {
            has_channels = true;
            let r = channel.reader(|buffer| Some(&buffers[buffer.index()]));
            if let Some(mut inputs) = r.read_inputs() {
                if let Some(first) = inputs.next() {
                    let mut last = first;
                    for t in inputs {
                        last = t;
                    }
                    first_t = first_t.min(first);
                    last_t = last_t.max(last);
                }
            }
            if let Some(ReadOutputs::Translations(v)) = r.read_outputs() {
                let mut prev: Option<Vec3> = None;
                for t in v {
                    let cur = vec3(t[0], t[1], t[2]);
                    if let Some(p) = prev {
                        motion_score += (cur - p).length();
                    }
                    prev = Some(cur);
                }
            }
        }
        let clip_duration = if first_t.is_finite() && last_t.is_finite() && last_t > first_t {
            last_t - first_t
        } else {
            0.0
        };
        let anim_name = anim.name().unwrap_or("").to_lowercase();
        let mut name_score = 0i32;
        if !anim_name.is_empty() && !stem_tokens.is_empty() {
            for t in &stem_tokens {
                if anim_name.contains(t) {
                    name_score += 1;
                }
            }
        }
        let metric = match mode {
            ClipSelectionMode::MostMotion => motion_score,
            ClipSelectionMode::LeastMotion => -motion_score,
            ClipSelectionMode::LongestDuration => clip_duration,
        };
        println!(
            "[anim] {} candidate anim={} name='{}' name_score={} duration={} motion={}",
            path,
            anim.index(),
            anim.name().unwrap_or(""),
            name_score,
            clip_duration,
            motion_score
        );
        if forced_clip_index.is_none() {
        if let Some(override_idx) = override_anim_index {
            if anim.index() == override_idx {
                if has_channels {
                    println!(
                        "[anim] {} using override anim index={} (stem='{}')",
                        path, override_idx, file_stem
                    );
                    selected_anim = Some(anim);
                    override_found = true;
                    break;
                }
                println!(
                    "[anim] {} override anim index={} has no channels; falling back to auto selection",
                    path, override_idx
                );
            }
        }
        }
        if has_channels
            && (name_score > best_name_score
                || (name_score == best_name_score && metric > best_metric)
                || (name_score == best_name_score
                    && (metric - best_metric).abs() <= f32::EPSILON
                    && clip_duration > best_duration))
        {
            best_name_score = name_score;
            best_metric = metric;
            best_duration = clip_duration;
            selected_anim = Some(anim);
        }
    }
    if forced_clip_index.is_none() && strict_override_mode && multi_clip_count > 1 && override_anim_index.is_none() {
        return Err(format!(
            "ANIM_REQUIRE_OVERRIDE is enabled and '{}' has {} clips but no override. \
Add one in animation_override_index_for_stem().",
            path, multi_clip_count
        ));
    }
    if forced_clip_index.is_none() && strict_override_mode && multi_clip_count > 1 && override_anim_index.is_some() && !override_found {
        return Err(format!(
            "ANIM_REQUIRE_OVERRIDE is enabled and '{}' override index {:?} was not found/usable.",
            path, override_anim_index
        ));
    }
    let anim = selected_anim.ok_or_else(|| format!("no usable animation clip found in '{}'", path))?;
    if forced_clip_index.is_none() && multi_clip_count > 1 && override_anim_index.is_none() {
        println!(
            "[anim] {} auto-selected anim={} (stem='{}'). To lock this clip, add override: \"{}\" => Some({}),",
            path,
            anim.index(),
            file_stem,
            file_stem,
            anim.index()
        );
    }
    Ok(anim)
}
