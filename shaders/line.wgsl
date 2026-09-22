struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color1: vec4<f32>,
    @location(2) color2: vec4<f32>,
    @location(3) geom1: vec4<f32>,
    @location(4) geom2: vec4<f32>,
    @location(5) geom3: vec4<f32>,
}

@group(0)
@binding(0)
var<uniform> transform: mat4x4<f32>;

@group(0)
@binding(1)
var<uniform> settings: vec4f;

@vertex
fn vs_main(
    @location(0) xyz: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color1: vec4<f32>,
    @location(3) color2: vec4<f32>,
    @location(4) geom1: vec4f,
    @location(5) geom2: vec4f,
    @location(6) geom3: vec4f
) -> VertexOutput {
    var result: VertexOutput;
    result.position = transform * vec4f(xyz, 1);
    result.uv = uv;
    result.color1 = color1;
    result.color2 = color2;
    result.geom1 = geom1;
    result.geom2 = geom2;
    result.geom3 = geom3;
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
    let shape = settings.x;
    var d = 0.0;
    if shape == 0.0 {
        d = min(vertex.geom1.x, vertex.geom1.y);
    } else if shape == 1.0 {
        let r = length(vertex.geom1.xy);
        let a = 1.0 - smoothstep(1.0 - vertex.geom1.w, 1.0, r);
        let b = smoothstep(vertex.geom1.z - vertex.geom1.w, vertex.geom1.z, r);
        d = a * b;
    } else if shape == 2.0 {
        // Round rectangle SDF
        let center = vertex.geom1.xy;
        // First select radius depending on which corner we are.
        let rad1h = select(vertex.geom2.xy, vertex.geom2.wz, vertex.position.x - vertex.geom1.x > 0);
        let rad1 = select(rad1h.x, rad1h.y, vertex.position.y - vertex.geom1.y > 0);
        let rad2h = select(vertex.geom3.xy, vertex.geom3.wz, vertex.position.x - vertex.geom1.x > 0);
        let rad2 = select(rad2h.x, rad2h.y, vertex.position.y - vertex.geom1.y > 0);
        // Now calculate the SDF for both outer and inner radius
        let size = vertex.geom1.zw - vec2f(rad1, rad1) - vec2f(0.5, 0.5);
        let k = length(max(vec2f(0, 0), abs(vertex.position.xy - center) - size));
        let d1 = 1.0 - smoothstep(rad1, rad1 + 1.0, k);
        let d2 = smoothstep(rad2, rad2 + 1.0, k);
        d = d2 * d1;
    } else {
        d = 0.5;
    }
    return vec4f(vertex.color1.rgb, d * vertex.color1.a);
}
