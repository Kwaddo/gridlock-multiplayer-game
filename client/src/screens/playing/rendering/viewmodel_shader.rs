use macroquad::material::{load_material, MaterialParams};
use macroquad::prelude::{Material, ShaderSource};

// Vertex shader: pass position, uv, color, normal and world position.
// Positions are transformed CPU-side before draw_mesh.
const VIEWMODEL_VERT: &str = r#"
#version 100
precision mediump float;

attribute vec3 in_position;
attribute vec2 in_uv;
attribute vec4 in_color;
attribute vec3 in_normal;

varying vec2 v_uv;
varying vec4 v_color;
varying vec3 v_normal;
varying vec3 v_world_pos;

uniform mat4 Projection;
uniform mat4 Model;

void main() {
    gl_Position = Projection * Model * vec4(in_position, 1.0);
    v_uv = in_uv;
    v_color = in_color;
    v_normal = normalize(mat3(Model) * in_normal);
    v_world_pos = (Model * vec4(in_position, 1.0)).xyz;
}
"#;

// Fragment shader: directional + fill + spec + rim lighting with gamma-correct output.
const VIEWMODEL_FRAG: &str = r#"
#version 100
precision mediump float;

varying vec2 v_uv;
varying vec4 v_color;
varying vec3 v_normal;
varying vec3 v_world_pos;

uniform sampler2D Texture;

vec3 srgb_to_linear(vec3 c) {
    return pow(max(c, vec3(0.0)), vec3(2.2));
}

vec3 linear_to_srgb(vec3 c) {
    return pow(max(c, vec3(0.0)), vec3(1.0 / 2.2));
}

void main() {
    vec3 light_dir = normalize(vec3(-0.4, 0.8, 0.5));
    vec3 fill_dir  = normalize(vec3(0.6, 0.2, -0.4));

    vec3 N = normalize(v_normal);
    vec3 V = normalize(-v_world_pos);

    float diff_key  = max(dot(N, light_dir), 0.0);
    float diff_fill = max(dot(N, fill_dir), 0.0) * 0.35;

    vec3 half_key = normalize(light_dir + V);
    float spec = pow(max(dot(N, half_key), 0.0), 28.0) * 0.25;

    float rim = pow(1.0 - max(dot(N, V), 0.0), 2.2) * 0.20;
    float ambient = 0.40;
    float light = ambient + diff_key * 0.55 + diff_fill + spec + rim;

    vec4 tex_color = texture2D(Texture, v_uv);
    vec3 base_srgb = v_color.rgb * tex_color.rgb;
    vec3 base_lin = srgb_to_linear(base_srgb);
    vec3 lit_lin = base_lin * light;
    vec3 out_srgb = linear_to_srgb(lit_lin);

    // Viewmodels are opaque by default to avoid accidental source alpha issues.
    gl_FragColor = vec4(out_srgb, 1.0);
}
"#;

pub fn create_viewmodel_material() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: VIEWMODEL_VERT,
            fragment: VIEWMODEL_FRAG,
        },
        MaterialParams::default(),
    )
    .expect("viewmodel shader failed to compile")
}
