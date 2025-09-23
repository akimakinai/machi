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

fn sample_color(layer: u32, pos: vec3<f32>, tri_w: vec3<f32>) -> vec4<f32> {
    // Triplanar texture mapping
    // https://qiita.com/edo_m18/items/c8995fe91778895c875e
    // https://ssr-maguro.hatenablog.com/entry/2020/01/22/192000
    let xy = textureSampleBias(my_array_texture, my_array_texture_sampler, pos.xy, layer, view.mip_bias);
    let yz = textureSampleBias(my_array_texture, my_array_texture_sampler, pos.yz, layer, view.mip_bias);
    let zx = textureSampleBias(my_array_texture, my_array_texture_sampler, pos.zx, layer, view.mip_bias);
    return xy * tri_w.z + yz * tri_w.x + zx * tri_w.y;
}

fn sample_normal(layer: u32, pos: vec3<f32>, tri_w: vec3<f32>, axis_s: vec3<f32>) -> vec3<f32> {
    // https://bgolus.medium.com/normal-mapping-for-a-triplanar-shader-10bf39dca05a
    let xy_nt = textureSampleBias(my_array_normal, my_array_normal_sampler, pos.xy, layer, view.mip_bias).rgb;
    let yz_nt = textureSampleBias(my_array_normal, my_array_normal_sampler, pos.yz, layer, view.mip_bias).rgb;
    let zx_nt = textureSampleBias(my_array_normal, my_array_normal_sampler, pos.zx, layer, view.mip_bias).rgb;
    return xy_nt.xyz * tri_w.z * axis_s.z +
           yz_nt.yzx * tri_w.x * axis_s.x +
           zx_nt.zxy * tri_w.y * axis_s.y;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var weights = pow(normalize(mesh.color), vec4(1.5));

    // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
    // the material members
    var pbr_input: PbrInput = pbr_input_new();
    let double_sided = (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        double_sided,
        is_front,
    );
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;

    let tri_weights: vec3<f32> = normalize(abs(mesh.world_normal));
    let axis_sign = sign(mesh.world_normal);

    var accum_color: vec4<f32> = vec4<f32>(0.0);
    var accum_normal: vec3<f32> = vec3<f32>(0.0);

    let eps = 0.0001;

    if (weights.x > eps) {
        let c = sample_color(0u, mesh.world_position.xyz, tri_weights);
        accum_color += c * weights.x;
        let n = sample_normal(0u, mesh.world_position.xyz, tri_weights, axis_sign);
        accum_normal += n * weights.x;
    }
    if (weights.y > eps) {
        let c = sample_color(1u, mesh.world_position.xyz, tri_weights);
        accum_color += c * weights.y;
        let n = sample_normal(1u, mesh.world_position.xyz, tri_weights, axis_sign);
        accum_normal += n * weights.y;
    }
    if (weights.z > eps) {
        let c = sample_color(2u, mesh.world_position.xyz, tri_weights);
        accum_color += c * weights.z;
        let n = sample_normal(2u, mesh.world_position.xyz, tri_weights, axis_sign);
        accum_normal += n * weights.z;
    }
    if (weights.w > eps) {
        let c = sample_color(3u, mesh.world_position.xyz, tri_weights);
        accum_color += c * weights.w;
        let n = sample_normal(3u, mesh.world_position.xyz, tri_weights, axis_sign);
        accum_normal += n * weights.w;
    }

    pbr_input.material.base_color = accum_color;
    pbr_input.N = normalize(accum_normal);
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    return tone_mapping(fns::apply_pbr_lighting(pbr_input), view.color_grading);
}
