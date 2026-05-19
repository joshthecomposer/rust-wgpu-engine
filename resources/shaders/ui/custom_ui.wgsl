struct UiUniforms {
	screen_size: vec2<f32>,
	_pad: vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: UiUniforms;

@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

struct VsIn {
	@location(0) position: vec2<f32>,
	@location(1) color: vec4<f32>,
	@location(2) uv: vec2<f32>,
	@location(3) rect_bounds: vec4<f32>,
	@location(4) border_radius: f32,
}

struct VsOut {
	@builtin(position) clip_pos: vec4<f32>,
	@location(0) color: vec4<f32>,
	@location(1) uv: vec2<f32>,
	@location(2) rect_bounds: vec4<f32>,
	@location(3) border_radius: f32,
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
	var out: VsOut;

	// pixel space (top-left origin) -> NDC; flip Y so screen-0 is at top.
	let ndc = (in.position / u.screen_size) * 2.0 - vec2<f32>(1.0, 1.0);
	out.clip_pos = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);

	out.color = in.color;
	out.uv = in.uv;
	out.rect_bounds = in.rect_bounds;
	out.border_radius = in.border_radius;
	return out;
}

// SDF rounded-rect discard mask. Returns true when the fragment should be
// kept. `frag_pos` is in the same top-left pixel space as `rect_bounds`.
fn sdf_keep(frag_pos: vec2<f32>, rect_bounds: vec4<f32>, border_radius: f32) -> bool {
	if (border_radius <= 0.0) {
		return true;
	}
	let rect_center = rect_bounds.xy + rect_bounds.zw * 0.5;
	let half_size = rect_bounds.zw * 0.5;

	let p = frag_pos - rect_center;
	let d = abs(p) - (half_size - vec2<f32>(border_radius, border_radius));
	let dist = length(max(d, vec2<f32>(0.0, 0.0)))
		+ min(max(d.x, d.y), 0.0)
		- border_radius;

	return dist <= 0.0;
}

// widget colors are authored in sRGB space (mentally, designers think "0.5 == mid
// gray on screen") The surface is an sRGB-encoded format, so the GPU
// applies linear to sRGB on write. To make a sRGB-authored value round-trip
// to its intended on-screen byte output the linear equivalent here.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
	let cutoff = step(c, vec3<f32>(0.04045));
	let lo = c / 12.92;
	let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
	return mix(hi, lo, cutoff);
}

@fragment
fn fs_solid(in: VsOut) -> @location(0) vec4<f32> {
	if (!sdf_keep(in.clip_pos.xy, in.rect_bounds, in.border_radius)) {
		discard;
	}
	let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
	let lin_rgb = srgb_to_linear(in.color.rgb * tex_color.rgb);
	return vec4<f32>(lin_rgb, in.color.a * tex_color.a);
}

@fragment
fn fs_alpha_mask(in: VsOut) -> @location(0) vec4<f32> {
	if (!sdf_keep(in.clip_pos.xy, in.rect_bounds, in.border_radius)) {
		discard;
	}
	let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
	let lin_rgb = srgb_to_linear(in.color.rgb);
	return vec4<f32>(lin_rgb, in.color.a * tex_color.r);
}
