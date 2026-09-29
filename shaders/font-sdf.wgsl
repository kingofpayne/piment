struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color1: vec4<f32>,
    @location(2) geom1: vec4<f32>
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
fn vs_main(@location(0) xyz: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color1: vec4<f32>, @location(4) geom1: vec4f) -> VertexOutput {
    var result: VertexOutput;
    result.position = transform * vec4f(xyz, 1);
    result.uv = uv;
    result.color1 = color1;
    result.geom1 = geom1;
    return result;
}

// The font atlas holds, in its red channel, the signed distance to the glyph contour, positive
// inside, remapped so the contour lies at 0.5.
//
// geom1.x is the distance range covered by the [0, 1] atlas values, in atlas texels.
// geom1.y moves the contour outwards, in pixels.
// geom1.z is the half width of the edge transition, in pixels.
@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
    let field = textureSample(texture, the_sampler, vertex.uv).r;
    // Screen pixels covered by one atlas texel, derived from the texture coordinates derivatives
    // so any glyph scaling is handled.
    let texels_per_pixel = fwidth(vertex.uv) * vec2f(textureDimensions(texture));
    let pixels_per_texel = 0.5 * dot(vec2f(1.0), 1.0 / texels_per_pixel);
    let distance = (field - 0.5) * vertex.geom1.x * pixels_per_texel + vertex.geom1.y;
    let alpha = smoothstep(-vertex.geom1.z, vertex.geom1.z, distance);
    return vec4(vertex.color1.rgb, alpha * vertex.color1.a);
}
