@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

struct CameraUniform {
	view_proj: mat4x4<f32>,
	inv_proj:  mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) uv:       vec2<f32>,
}

struct InstanceInput {
	@location(2) mm0:   vec4<f32>,
	@location(3) mm1:   vec4<f32>,
	@location(4) mm2:   vec4<f32>,
	@location(5) mm3:   vec4<f32>,
	@location(6) color: vec4<f32>,
	@location(7) alpha: f32,
	@location(8) flags: u32,
}

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) uv:                              vec2<f32>,
	@location(1) color:                           vec4<f32>,
	@location(2) alpha:                           f32,
	@location(3) @interpolate(flat) flags:        u32,
}

struct FragmentOut {
	@location(0) color:  vec4<f32>,
	@location(1) bright: vec4<f32>,
}

const FLAG_HAS_TEX:       u32 = 1u;
const FLAG_TEX_HAS_ALPHA: u32 = 2u;
const FLAG_HAS_BLOOM:     u32 = 4u;

@vertex
fn vs_main(v: VertexInput, i: InstanceInput) -> VertexOutput {
	let model_matrix = mat4x4<f32>(i.mm0, i.mm1, i.mm2, i.mm3);

	var out: VertexOutput;
	out.clip_position = camera.view_proj * model_matrix * vec4<f32>(v.position, 1.0);
	out.uv    = v.uv;
	out.color = i.color;
	out.alpha = i.alpha;
	out.flags = i.flags;
	return out;
}

fn extract_bright(color: vec3<f32>, threshold: f32, knee: f32) -> vec3<f32> {
	let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));

	let soft = luma - threshold + knee;
	let clamped_soft = clamp(soft, 0.0, 2.0 * knee);
	let curve = (clamped_soft * clamped_soft) / (4.0 * knee + 1e-5);

	let contribution = max(curve, luma - threshold);
	let scale = contribution / max(luma, 1e-5);
	return color * scale;
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOut {
	let has_tex       = (in.flags & FLAG_HAS_TEX)       != 0u;
	let tex_has_alpha = (in.flags & FLAG_TEX_HAS_ALPHA) != 0u;
	let has_bloom     = (in.flags & FLAG_HAS_BLOOM)     != 0u;

	// `textureSample` computes implicit derivatives, so it must run in uniform
	// control flow. Sample unconditionally and fall back to white for untextured
	// particles instead of sampling inside the `has_tex` branch.
	let sampled = textureSample(t_diffuse, s_diffuse, in.uv);
	let t = select(vec4<f32>(1.0, 1.0, 1.0, 1.0), sampled, has_tex);

	var mask: f32;
	if (tex_has_alpha) {
		mask = select(1.0, t.a, has_tex);
	} else {
		mask = select(1.0, dot(t.rgb, vec3<f32>(0.299, 0.587, 0.114)), has_tex);
	}

	let out_a = in.color.a * in.alpha * mask;
	if (out_a <= 0.001) {
		discard;
	}

	let lit = vec4<f32>(in.color.rgb, out_a);

	var out: FragmentOut;
	out.color = lit;

	if (has_bloom) {
		out.bright = vec4<f32>(extract_bright(lit.rgb, 1.0, 0.5), lit.a);
	} else {
		out.bright = vec4<f32>(0.0, 0.0, 0.0, 0.0);
	}
	return out;
}
