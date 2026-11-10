use super::{AnimatedModel, Pose};

/*
    which bones count as upper body for layering (aiming, firing on top of walking)
    spine and everything above it, eased in so the waist twists instead of snapping
    
*/
const DEPTH: [(&str, f32); 3] = [("spine", 0.5), ("spine1", 0.8), ("spine2", 1.0)];

// per node how much the upper layer takes over, 0 = legs/hips, 1 = full upper body
pub(super) fn upper_body_mask(node_names: &[String], parents: &[Option<usize>]) -> Vec<f32> {
    let weight_of = |i: usize| {
        DEPTH.iter().find(|(b, _)| node_names[i].rsplit(':').next() == Some(*b)).map(|d| d.1)
    };
    (0..node_names.len())
        .map(|mut i| loop {
            if let Some(w) = weight_of(i) {
                break w; // nearest spine bone at or above this node decides
            }
            match parents[i] {
                Some(p) => i = p,
                None => break 0.0,
            }
        })
        .collect()
}

impl AnimatedModel {
    // mix layer into pose on the upper body only, 'w' fades the whole layer in and out
    pub fn layer_upper_body(&self, pose: &mut Pose, layer: &Pose, w: f32) {
        pose.blend_masked_from(layer, &self.upper_mask, w);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // hips(0) -> spine(1) -> spine1(2) -> spine2(3) -> neck(4) -> head(5), leg(6) off the hips
    fn rig() -> (Vec<String>, Vec<Option<usize>>) {
        let names = ["mixamorig:hips", "mixamorig:spine", "mixamorig:spine1", "mixamorig:spine2", "mixamorig:neck", "mixamorig:head", "mixamorig:leftupleg"];
        (names.iter().map(|n| n.to_string()).collect(), vec![None, Some(0), Some(1), Some(2), Some(3), Some(4), Some(0)])
    }

    #[test]
    fn mask_eases_in_up_the_spine() {
        let (names, parents) = rig();
        assert_eq!(upper_body_mask(&names, &parents), vec![0.0, 0.5, 0.8, 1.0, 1.0, 1.0, 0.0]);
    }

    // two poses on the rig: base all at rest, layer turned 1 radian on every bone
    fn poses() -> (Pose, Pose) {
        use macroquad::prelude::*;
        let rest = super::super::LocalTransform { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };
        let base = Pose::new(&vec![rest; 7], 0);
        let mut layer = Pose::new(&vec![rest; 7], 0);
        for t in &mut layer.local {
            t.rotation = Quat::from_rotation_x(1.0);
        }
        (base, layer)
    }

    #[test]
    fn masked_blend_moves_only_the_upper_body() {
        let (names, parents) = rig();
        let mask = upper_body_mask(&names, &parents);
        let (mut base, layer) = poses();
        base.blend_masked_from(&layer, &mask, 1.0);
        let turned: Vec<f32> = base.local.iter().map(|t| t.rotation.to_axis_angle().1).collect();
        let want = [0.0, 0.5, 0.8, 1.0, 1.0, 1.0, 0.0]; // radians, = the mask
        for (got, want) in turned.iter().zip(want) {
            assert!((got - want).abs() < 1e-4, "{turned:?}");
        }
    }

    #[test]
    fn layer_weight_zero_changes_nothing() {
        let (names, parents) = rig();
        let mask = upper_body_mask(&names, &parents);
        let (mut base, layer) = poses();
        base.blend_masked_from(&layer, &mask, 0.0);
        assert!(base.local.iter().all(|t| t.rotation.abs_diff_eq(macroquad::prelude::Quat::IDENTITY, 1e-6)));
    }

    #[test]
    fn real_character_arms_and_head_are_upper_legs_are_not() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models/characters/Ch15_humanoid.glb");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("no converted assets, skipping");
            return;
        };
        let (doc, _, _) = super::super::glb::import_glb("Ch15", &bytes).unwrap();
        let names: Vec<String> = doc.nodes().map(|n| n.name().unwrap_or("").to_ascii_lowercase()).collect();
        let mut parents = vec![None; names.len()];
        for node in doc.nodes() {
            for child in node.children() {
                parents[child.index()] = Some(node.index());
            }
        }
        let mask = upper_body_mask(&names, &parents);
        let weight = |bone: &str| names.iter().position(|n| n.rsplit(':').next() == Some(bone)).map(|i| mask[i]);
        for upper in ["head", "leftarm", "righthand", "neck"] {
            assert_eq!(weight(upper), Some(1.0), "{upper}");
        }
        for lower in ["hips", "leftupleg", "rightfoot"] {
            assert_eq!(weight(lower), Some(0.0), "{lower}");
        }
    }
}
