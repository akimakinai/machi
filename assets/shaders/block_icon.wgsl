#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0)
var block_texture: texture_2d<f32>;
@group(1) @binding(1)
var block_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let mx = select(1.0 - in.uv.x, in.uv.x, in.uv.x < 0.5);

    let x_bound = 0.5 - sqrt(3.0) / 4.0;

    if mx < x_bound {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let l1y = line1(mx);
    let l2y = line2(mx);
    let l3y = line3(mx);

    var map_uv: vec2<f32>;
    if l2y > in.uv.y && in.uv.y >= l1y {
        // top
        map_uv = mapping_uv(vec2(x_bound, line1(x_bound)), vec2(0.5, 0.0), vec2(0.5, line2(0.5)), in.uv);
    } else if l3y >= in.uv.y && in.uv.y >= l2y {
        if in.uv.x < 0.5 {
            // left
            map_uv = mapping_uv(vec2(x_bound, line1(x_bound)), vec2(0.5, line2(0.5)), vec2(x_bound, line3(x_bound)), in.uv);
        } else {
            // right
            map_uv = mapping_uv(vec2(0.5, 0.0), vec2(1.0 - x_bound, line1(x_bound)), vec2(0.5, 1.0), in.uv);
        }
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    return textureSample(block_texture, block_sampler, map_uv);
}

//          === line1    
//      ====             
//   ===                 
//   |  ====             
//   |      === line2    
//   |                   
//   |                   
//   |                   
//    ===                
//       ====            
//           == line3    

fn line1(x: f32) -> f32 {
    return 0.5 / sqrt(3.0) - x / sqrt(3.0);
}

fn line2(x: f32) -> f32 {
    return 0.5 - 0.5 / sqrt(3.0) + x / sqrt(3.0);
}

fn line3(x: f32) -> f32 {
    return 1.0 - 0.5 / sqrt(3.0) + x / sqrt(3.0);
}

fn mapping_uv(top_left: vec2<f32>, top_right: vec2<f32>, bottom_left: vec2<f32>, pos: vec2<f32>) -> vec2<f32> {
    let u = reverse_mix(top_left, top_right, pos);
    let v = reverse_mix(bottom_left, top_left, pos);
    return vec2<f32>(u, v);
}

fn reverse_mix(a: vec2<f32>, b: vec2<f32>, x: vec2<f32>) -> f32 {
    return clamp(dot(x - a, b - a) / length(b - a), 0.0, 1.0);
}
