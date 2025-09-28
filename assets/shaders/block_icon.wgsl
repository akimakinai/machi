#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0)
var color_texture: texture_2d<f32>;
@group(1) @binding(1)
var color_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let mx = select(1.0 - in.uv.x, in.uv.x, in.uv.x < 0.5);

    if mx < (0.5 - sqrt(3.0) / 4.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let l1y = 0.5 / sqrt(3.0) - mx / sqrt(3.0);
    let l2y = 0.5 - 0.5 / sqrt(3.0) + mx / sqrt(3.0);
    let l3y = 1.0 - 0.5 / sqrt(3.0) + mx / sqrt(3.0);

    if l2y > in.uv.y && in.uv.y >= l1y {
        // top
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    } else if l3y >= in.uv.y && in.uv.y >= l2y {
        if in.uv.x < 0.5 {
            // left
            return vec4<f32>(0.0, 1.0, 0.0, 1.0);
        } else {
            // right
            return vec4<f32>(0.0, 0.0, 1.0, 1.0);
        }
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
}
