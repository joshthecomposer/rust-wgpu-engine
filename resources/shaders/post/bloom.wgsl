struct BloomParams {
    texel_size: vec2<f32>,
    _pad0: vec2<f32>,
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;
@group(0) @binding(2) var<uniform> params: BloomParams;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let uv = vec2<f32>(
        f32((vi << 1u) & 2u),
        f32(vi & 2u),
    );
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    out.clip_pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

@fragment
fn fs_downsample(in: VsOut) -> @location(0) vec4<f32> {
    let t = params.texel_size;
    let uv = in.uv;

    let a = textureSample(src_tex, src_sampler, uv);
    let b = textureSample(src_tex, src_sampler, uv + vec2<f32>(-0.5, -0.5) * t);
    let c = textureSample(src_tex, src_sampler, uv + vec2<f32>( 0.5, -0.5) * t);
    let d = textureSample(src_tex, src_sampler, uv + vec2<f32>(-0.5,  0.5) * t);
    let e = textureSample(src_tex, src_sampler, uv + vec2<f32>( 0.5,  0.5) * t);
    let f = textureSample(src_tex, src_sampler, uv + vec2<f32>(-1.0, -1.0) * t);
    let g = textureSample(src_tex, src_sampler, uv + vec2<f32>( 0.0, -1.0) * t);
    let h = textureSample(src_tex, src_sampler, uv + vec2<f32>( 1.0, -1.0) * t);
    let i = textureSample(src_tex, src_sampler, uv + vec2<f32>(-1.0,  0.0) * t);
    let j = textureSample(src_tex, src_sampler, uv + vec2<f32>( 1.0,  0.0) * t);
    let k = textureSample(src_tex, src_sampler, uv + vec2<f32>(-1.0,  1.0) * t);
    let l = textureSample(src_tex, src_sampler, uv + vec2<f32>( 0.0,  1.0) * t);
    let m = textureSample(src_tex, src_sampler, uv + vec2<f32>( 1.0,  1.0) * t);

    var color = a * 0.125;
    color += (b + c + d + e) * 0.125;
    color += (f + g + i) * 0.0625;
    color += (g + h + j) * 0.0625;
    color += (i + k + l) * 0.0625;
    color += (j + l + m) * 0.0625;

    return vec4<f32>(color.rgb, 1.0);
}

@fragment
fn fs_upsample(in: VsOut) -> @location(0) vec4<f32> {
    let t = params.texel_size;
    let uv = in.uv;

    var color = textureSample(src_tex, src_sampler, uv + vec2<f32>(-1.0, -1.0) * t) * 0.0625;
    color += textureSample(src_tex, src_sampler, uv + vec2<f32>( 0.0, -1.0) * t) * 0.125;
    color += textureSample(src_tex, src_sampler, uv + vec2<f32>( 1.0, -1.0) * t) * 0.0625;
    color += textureSample(src_tex, src_sampler, uv + vec2<f32>(-1.0,  0.0) * t) * 0.125;
    color += textureSample(src_tex, src_sampler, uv) * 0.25;
    color += textureSample(src_tex, src_sampler, uv + vec2<f32>( 1.0,  0.0) * t) * 0.125;
    color += textureSample(src_tex, src_sampler, uv + vec2<f32>(-1.0,  1.0) * t) * 0.0625;
    color += textureSample(src_tex, src_sampler, uv + vec2<f32>( 0.0,  1.0) * t) * 0.125;
    color += textureSample(src_tex, src_sampler, uv + vec2<f32>( 1.0,  1.0) * t) * 0.0625;

    return vec4<f32>(color.rgb, 1.0);
}
