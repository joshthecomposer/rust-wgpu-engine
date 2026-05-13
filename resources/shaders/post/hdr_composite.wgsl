struct HdrCompositeParams {
    exposure: f32,
    bloom_strength: f32,
    hdr_enabled: u32,
    _pad0: u32,
	inv_proj: mat4x4<f32>,
}

@group(0) @binding(0) var scene_hdr: texture_2d<f32>;
@group(0) @binding(1) var scene_hdr_sampler: sampler;
@group(0) @binding(2) var<uniform> params: HdrCompositeParams;
@group(0) @binding(3) var depth_tex: texture_depth_2d;
@group(0) @binding(4) var bloom_tex: texture_2d<f32>;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    // Fullscreen triangle
    let uv = vec2<f32>(
        f32((vi << 1u) & 2u),
        f32(vi & 2u),
    );
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    out.clip_pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

const FOG_COLOR: vec3<f32> = vec3<f32>(0.05, 0.05, 0.06);
const FOG_START: f32 = 5.0;
const FOG_END:   f32 = 85.0;
const FOG_STRENGTH: f32 = 0.95;

fn view_z_from_depth(depth01: f32, uv: vec2<f32>) -> f32 {
    // perspective_rh maps view-space z to NDC depth [0..1] (WebGPU/Vulkan convention).
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth01, 1.0);
    let view = params.inv_proj * ndc;
    return -view.z / view.w;
}

fn fog_factor(dist: f32) -> f32 {
    let f = (dist - FOG_START) / max(FOG_END - FOG_START, 0.000001);
    return clamp(f, 0.0, 1.0) * FOG_STRENGTH;
}


@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var color = textureSample(scene_hdr, scene_hdr_sampler, in.uv).rgb;
    let bloom = textureSample(bloom_tex, scene_hdr_sampler, in.uv).rgb;
    color += bloom * params.bloom_strength;

    let depth = textureLoad(depth_tex, vec2<i32>(in.clip_pos.xy), 0);

    var ff = fog_factor(view_z_from_depth(depth, in.uv));

    if (depth >= 0.999999) {
        ff = 0.85;
    }

    color = mix(color, FOG_COLOR, ff);

    if (params.hdr_enabled != 0u) {
        color = vec3<f32>(1.0) - exp(-color * max(params.exposure, 0.0001));
    }

    return vec4<f32>(color, 1.0);
}
