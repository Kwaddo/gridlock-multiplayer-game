use super::*;

/// One triangle-list primitive of a test rig: `triangles` unshared
/// triangles, skinned or rigid, with material `material`.
struct Prim {
    triangles: usize,
    skinned: bool,
    material: Option<usize>,
}

/// A .glb with one joint, a one-channel clip and `prims` in one mesh. Each
/// material `i` of `materials` takes its colour from image `materials[i]`
/// (bytes that are no image: only their indices are looked at).
fn rig(prims: &[Prim], materials: &[usize]) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut view = |bin: &mut Vec<u8>, data: Vec<u8>| {
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        views.push(format!(r#"{{"buffer":0,"byteOffset":{},"byteLength":{}}}"#, bin.len(), data.len()));
        bin.extend_from_slice(&data);
        views.len() - 1
    };
    let f32s = |v: &[f32]| v.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
    let mut accessor = |view: usize, component: u32, count: usize, kind: &str, bounds: &str| {
        accessors.push(format!(
            r#"{{"bufferView":{view},"componentType":{component},"count":{count},"type":"{kind}"{bounds}}}"#
        ));
        accessors.len() - 1
    };
    let mut primitives = Vec::new();
    for prim in prims {
        let verts = prim.triangles * 3;
        let positions: Vec<f32> = (0..verts).flat_map(|i| [(i % 3) as f32, (i / 3) as f32 * 0.01, 0.0]).collect();
        let top = (verts / 3) as f32 * 0.01;
        let pos_view = view(&mut bin, f32s(&positions));
        let bounds = format!(r#","min":[0,0,0],"max":[2,{top},0]"#);
        let pos = accessor(pos_view, 5126, verts, "VEC3", &bounds);
        let idx_view = view(&mut bin, (0..verts as u16).flat_map(|i| i.to_le_bytes()).collect());
        let idx = accessor(idx_view, 5123, verts, "SCALAR", "");
        let mut attributes = format!(r#""POSITION":{pos}"#);
        if prim.skinned {
            let joints_view = view(&mut bin, vec![0u8; verts * 8]);
            let joints = accessor(joints_view, 5123, verts, "VEC4", "");
            let weights_view = view(&mut bin, f32s(&[1.0, 0.0, 0.0, 0.0].repeat(verts)));
            let weights = accessor(weights_view, 5126, verts, "VEC4", "");
            attributes += &format!(r#","JOINTS_0":{joints},"WEIGHTS_0":{weights}"#);
        }
        let material = prim.material.map(|m| format!(r#","material":{m}"#)).unwrap_or_default();
        primitives.push(format!(r#"{{"attributes":{{{attributes}}},"indices":{idx}{material}}}"#));
    }
    let times_view = view(&mut bin, f32s(&[0.0, 1.0]));
    let times = accessor(times_view, 5126, 2, "SCALAR", r#","min":[0],"max":[1]"#);
    let moves_view = view(&mut bin, f32s(&[0.0, 0.0, 0.0, 0.0, 1.0, 0.0]));
    let moves = accessor(moves_view, 5126, 2, "VEC3", "");
    let image_count = materials.iter().map(|&i| i + 1).max().unwrap_or(0);
    let images: Vec<String> = (0..image_count)
        .map(|_| format!(r#"{{"bufferView":{},"mimeType":"image/png"}}"#, view(&mut bin, vec![1, 2, 3, 4])))
        .collect();
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let textures: Vec<String> = (0..image_count).map(|i| format!(r#"{{"source":{i}}}"#)).collect();
    let materials: Vec<String> = materials
        .iter()
        .map(|&i| format!(r#"{{"pbrMetallicRoughness":{{"baseColorTexture":{{"index":{i}}}}}}}"#))
        .collect();
    let json = format!(
        r#"{{"asset":{{"version":"2.0"}},"buffers":[{{"byteLength":{}}}],"bufferViews":[{}],"accessors":[{}],
        "meshes":[{{"primitives":[{}]}}],"materials":[{}],"textures":[{}],"images":[{}],
        "nodes":[{{"name":"mixamorig:Hips"}},{{"mesh":0,"skin":0}}],"skins":[{{"joints":[0]}}],
        "animations":[{{"channels":[{{"sampler":0,"target":{{"node":0,"path":"translation"}}}}],
        "samplers":[{{"input":{times},"output":{moves}}}]}}],"scenes":[{{"nodes":[0,1]}}]}}"#,
        bin.len(),
        views.join(","),
        accessors.join(","),
        primitives.join(","),
        materials.join(","),
        textures.join(","),
        images.join(","),
    );
    let mut json = json.into_bytes();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&((12 + 8 + json.len() + 8 + bin.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    out
}

fn meshes(model: &AnimatedModel) -> Vec<(Vec<Vec3>, Vec<u16>)> {
    model.meshes().map(|m| (m.vertices.iter().map(|v| v.position).collect(), m.indices.clone())).collect()
}

/// A build takes three steps (parts, split, finish), each giving nothing
/// until the last gives the model, the one the one-shot loader gives.
#[test]
fn a_build_steps_through_its_stages() {
    // 2,000 triangles: over the draw limit, so the split cuts them up
    let skinned = |triangles| Prim { triangles, skinned: true, material: None };
    let bytes = rig(&[skinned(2000), skinned(10)], &[]);
    let (doc, buffers, images) = import_glb("rig", &bytes).unwrap();
    let mode = ClipSelectionMode::LongestDuration;
    let mut build = AnimatedBuild::start("rig", &doc, &buffers, mode, None, false).unwrap();
    assert!(build.texture_images(&doc, &buffers).is_empty());
    assert!(matches!(build.step(&doc, &buffers, &images), Ok(None)), "parts");
    assert!(matches!(&build.stage, Stage::Split(parts, _) if parts.len() == 2));
    assert!(matches!(build.step(&doc, &buffers, &images), Ok(None)), "split");
    assert!(matches!(&build.stage, Stage::Finish(parts, _) if parts.len() > 2));
    let stepped = build.step(&doc, &buffers, &images).unwrap().expect("the model, last");
    assert!(build.step(&doc, &buffers, &images).is_err(), "over");

    let whole = load_gltf_animated_model("rig", &bytes).unwrap();
    assert_eq!(meshes(&stepped), meshes(&whole));
    assert_eq!(stepped.total_vertex_count(), 2010 * 3);
    assert_eq!(stepped.duration(), whole.duration());
    assert_eq!(stepped.joint_names, whole.joint_names);
}

/// What `start` refuses, the one-shot loader fails on with the same error.
#[test]
fn start_checks_what_the_loader_checks() {
    let rigid = Prim { triangles: 1, skinned: false, material: None };
    let bytes = rig(&[rigid], &[]);
    let (doc, buffers, _) = import_glb("rig", &bytes).unwrap();
    let mode = ClipSelectionMode::LongestDuration;
    let err = AnimatedBuild::start("rig", &doc, &buffers, mode, None, false).err().unwrap();
    assert!(err.starts_with("no skinned primitive"), "{err}");
    assert_eq!(load_gltf_animated_model("rig", &bytes).err(), Some(err));
}

/// The images uploaded ahead are the base colour images of the primitives
/// built, each once, in the order the build asks for them.
#[test]
fn texture_images_are_those_the_build_uses() {
    let prim = |skinned, material| Prim { triangles: 1, skinned, material: Some(material) };
    // material 0 is image 2, 1 is image 0, 2 is image 1
    let bytes = rig(&[prim(true, 0), prim(false, 1), prim(true, 0), prim(true, 2)], &[2, 0, 1]);
    let (doc, buffers, _) = import_glb("rig", &bytes).unwrap();
    let mode = ClipSelectionMode::LongestDuration;
    let build = AnimatedBuild::start("rig", &doc, &buffers, mode, None, false).unwrap();
    assert_eq!(build.texture_images(&doc, &buffers), vec![2, 1]);
    let build = AnimatedBuild::start("rig", &doc, &buffers, mode, None, true).unwrap();
    assert_eq!(build.texture_images(&doc, &buffers), vec![2, 1, 0], "skinned first, then rigid");
}
