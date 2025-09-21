// Based on https://github.com/bevyengine/bevy/blob/45f54fd884b72c78407943597ffc4ee7c5d22dac/assets/shaders/array_texture.wgsl
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_types::{STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT, PbrInput, pbr_input_new},
    pbr_functions as fns,
    pbr_bindings,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var my_array_texture: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var my_array_texture_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var my_array_normal: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var my_array_normal_sampler: sampler;

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    let layer = (i32(mesh.world_position.x) + i32(mesh.world_position.z)) & 0x3;

    // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
    // the material members
    var pbr_input: PbrInput = pbr_input_new();

    // Tri-planar mapping based on:
    // https://qiita.com/edo_m18/items/c8995fe91778895c875e
    // https://ssr-maguro.hatenablog.com/entry/2020/01/22/192000
    var blending: vec3<f32> = normalize(abs(mesh.world_normal));

    var xy_tex = textureSampleBias(my_array_texture, my_array_texture_sampler, mesh.world_position.xy, layer, view.mip_bias);
    var yz_tex = textureSampleBias(my_array_texture, my_array_texture_sampler, mesh.world_position.yz, layer, view.mip_bias);
    var zx_tex = textureSampleBias(my_array_texture, my_array_texture_sampler, mesh.world_position.zx, layer, view.mip_bias);

    pbr_input.material.base_color = xy_tex * blending.z + yz_tex * blending.x + zx_tex * blending.y;
// #ifdef VERTEX_COLORS
//     pbr_input.material.base_color = pbr_input.material.base_color * mesh.color;
// #endif

    let double_sided = (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        double_sided,
        is_front,
    );

    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;

    let xy_nt = textureSampleBias(my_array_normal, my_array_normal_sampler, mesh.world_position.xy, layer, view.mip_bias).rgb;
    let yz_nt = textureSampleBias(my_array_normal, my_array_normal_sampler, mesh.world_position.yz, layer, view.mip_bias).rgb;
    let zx_nt = textureSampleBias(my_array_normal, my_array_normal_sampler, mesh.world_position.zx, layer, view.mip_bias).rgb;

    // https://bgolus.medium.com/normal-mapping-for-a-triplanar-shader-10bf39dca05a
    let axis_sign = sign(mesh.world_normal);
    pbr_input.N = normalize(xy_nt.xyz * blending.z * axis_sign.z +
                            yz_nt.yzx * blending.x * axis_sign.x +
                            zx_nt.zxy * blending.y * axis_sign.y);

    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    return tone_mapping(fns::apply_pbr_lighting(pbr_input), view.color_grading);
}
