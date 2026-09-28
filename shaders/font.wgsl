struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color1: vec4<f32>,
    @location(2) color2: vec4<f32>
}

@group(0)
@binding(0)
var<uniform> transform: mat4x4<f32>;

@group(0)
@binding(2)
var texture: texture_2d<f32>;

@group(0)
@binding(3)
var the_sampler: sampler;

@vertex
fn vs_main(@location(0) xyz: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color1: vec4<f32>, @location(3) color2: vec4f) -> VertexOutput {
    var result: VertexOutput;
    result.position = transform * vec4f(xyz, 1);
    result.uv = uv;
    result.color1 = color1;
    result.color2 = color2;
    return result;
}

// The font atlas holds the plain characters in the red channel, their outlines in the green one and
// their blurred shadows in the blue one. color2 is a mask selecting one of these channels as the
// alpha of the color1 drawing color.
@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
    let tex = textureSample(texture, the_sampler, vertex.uv);
    let masked = tex.rgb * vertex.color2.rgb;
    let alpha = max(masked.r, max(masked.g, masked.b));
    return vec4(vertex.color1.rgb, alpha * vertex.color1.a);
}
