struct SkyCamera {
    view_rot: mat4x4<f32>,
    proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> sky_cam: SkyCamera;

@group(1) @binding(0)
var env_map: texture_cube<f32>;
@group(1) @binding(1)
var env_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec3<f32>,
}

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;

    let p = sky_cam.proj * sky_cam.view_rot * vec4<f32>(position, 1.0);
    // Same idea as gl_Position = vec4(pos.xy, pos.w, pos.w): NDC z → 1 (far), works with depth cleared to 1 + LessEqual.
    out.clip_position = vec4<f32>(p.x, p.y, p.w, p.w);

    out.tex_coords = vec3<f32>(position.x, position.y, -position.z);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = textureSample(env_map, env_sampler, normalize(in.tex_coords));
    return c;
}
