#import bevy_pbr::{
    pbr_types,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
}

@group(3) @binding(100) var<uniform> variation: vec4<f32>;

@fragment
fn fragment(vertex_output: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(vertex_output, is_front);

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    let normal = normalize(pbr_input.world_normal);
    let world_pos = vertex_output.world_position.xyz;

    let pos_hash = fract(sin(dot(world_pos.xz, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let height_var = smoothstep(0.0, 80.0, world_pos.y);
    let normal_weight = abs(normal.y) * variation.y + (1.0 - variation.y);
    let color_var = mix(1.0 - variation.x, 1.0 + variation.x, pos_hash * 0.3) * normal_weight * mix(1.0 - variation.z, 1.0 + variation.z, height_var);

    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb * color_var,
        pbr_input.material.base_color.a
    );

    var out: FragmentOutput;
    if ((pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}