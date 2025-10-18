#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0)
var block_texture: texture_2d<f32>;
@group(1) @binding(1)
var block_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(block_texture, block_sampler, in.uv);
    if tex.a > 0.5 {
        return tex;
    }

    // Draw drop shadow
    let duv = in.uv - vec2(0.04);
    return vec4<f32>(0.0, 0.0, 0.0, textureSample(block_texture, block_sampler, duv).a * 0.8);
}
