@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

struct CameraUniform {
	view_proj: mat4x4<f32>,
	inv_proj: mat4x4<f32>,
	light_view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct DirLightUniform {
	direction: vec4<f32>,
	view_pos: vec4<f32>,
	ambient: vec4<f32>,
	diffuse: vec4<f32>,
	specular: vec4<f32>,
}
struct ShadowSampleUniform {
	bias_scalar: f32,
	border_fallback: u32,
	_pad0: u32,
	_pad1: u32,
}
@group(2) @binding(0)
var<uniform> dir_light: DirLightUniform;
@group(2) @binding(1)
var shadow_map: texture_depth_2d;
@group(2) @binding(2)
var shadow_sampler: sampler;
@group(2) @binding(3)
var<uniform> shadow: ShadowSampleUniform;

struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) normal: vec3<f32>,
	@location(2) uv: vec2<f32>, 
	@location(3) bone_ids: vec4<i32>,
	@location(4) bone_weights: vec4<f32>
}

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
	@location(1) normal: vec4<f32>,
	@location(2) world_position: vec4<f32>,
	@location(3) frag_pos_light_space: vec4<f32>,
}

struct InstanceInput {
	@location(5) mm0: vec4<f32>,
	@location(6) mm1: vec4<f32>,
	@location(7) mm2: vec4<f32>,
	@location(8) mm3: vec4<f32>,
}

@vertex
fn vs_main(
	model: VertexInput,
	instance: InstanceInput,
) -> VertexOutput {
	var out: VertexOutput;

	let model_matrix = mat4x4<f32>(
		instance.mm0,
		instance.mm1,
		instance.mm2,
		instance.mm3
	);

	let world_pos = model_matrix * vec4<f32>(model.position, 1.0);
	let world_normal = normalize((model_matrix * vec4<f32>(model.normal, 0.0)).xyz);

	out.world_position = world_pos;
	out.normal = vec4<f32>(world_normal, 0.0);
	out.clip_position = camera.view_proj * world_pos;
	out.frag_pos_light_space = camera.light_view_proj * world_pos;
	out.uv = model.uv;

	return out;
}

struct FragmentOut {
	@location(0) color: vec4<f32>,
	@location(1) bright: vec4<f32>,
	@location(2) depth_proxy: vec4<f32>,
}

struct FragmentVaryings {
	@location(0) uv: vec2<f32>,
	@location(1) normal: vec4<f32>,
	@location(2) world_position: vec4<f32>,
	@location(3) frag_pos_light_space: vec4<f32>,
}

fn shadow_calculation(frag_pos_light_space: vec4<f32>, dot_light_normal: f32) -> f32 {
	let w = max(frag_pos_light_space.w, 1e-6);
	let ndc = frag_pos_light_space.xyz / w;
	var pos = vec3<f32>(ndc.x * 0.5 + 0.5, 1.0 - (ndc.y * 0.5 + 0.5), ndc.z);
	if (pos.x < 0.0 || pos.x > 1.0 || pos.y < 0.0 || pos.y > 1.0 || pos.z < 0.0 || pos.z > 1.0) {
		return 1.0;
	}

	let bias = max(shadow.bias_scalar * (1.0 - dot_light_normal), 0.0005);
	let dims = textureDimensions(shadow_map, 0);
	let texel = 1.0 / vec2<f32>(dims);
	var sum = 0.0;

	for (var x = -1; x <= 1; x++) {
		for (var y = -1; y <= 1; y++) {
			let uv = pos.xy + vec2<f32>(f32(x), f32(y)) * texel;
			if (shadow.border_fallback != 0u
				&& (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0)) {
				sum += 1.0;
			} else {
				let stored = textureLoad(
					shadow_map,
					vec2<i32>(clamp(uv * vec2<f32>(dims), vec2(0.0), vec2<f32>(dims) - vec2(1.0))),
					0,
				);
				sum += select(1.0, 0.0, stored + bias < pos.z);
			}
		}
	}
	return sum / 9.0;
}

fn calculate_directional_light(in: FragmentVaryings) -> vec4<f32> {
	let light_color = dir_light.diffuse;
	let view_position = dir_light.view_pos;
	
	let view_dist = length(view_position.xyz - in.world_position.xyz);
	let lod = clamp((view_dist - 5.0) / 5.0, 0.0, 30.0);
	let tex_sample = textureSampleLevel(t_diffuse, s_diffuse, in.uv, lod);
	
	if (tex_sample.a < 0.5) {
		discard;
	}

	let flat_ambient = dir_light.ambient;
	let light_dir = normalize(dir_light.direction.xyz);
	let norm = normalize(in.normal.xyz);

	let dot_light_normal = dot(light_dir, norm);
	let diff = max(dot_light_normal, 0.0);
	let diffuse = diff * light_color;

	let s = shadow_calculation(in.frag_pos_light_space, dot_light_normal);

	return vec4<f32>((flat_ambient.rgb + s * diffuse.rgb) * tex_sample.rgb, tex_sample.a);

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
fn fs_main(@builtin(position) frag_pos: vec4<f32>, in: FragmentVaryings) -> FragmentOut {
	let lit = calculate_directional_light(in);

	var out: FragmentOut;

	out.color = lit;

	out.bright = vec4<f32>(extract_bright(lit.rgb, 1.0, 0.5), lit.a);
	out.depth_proxy = vec4<f32>(frag_pos.z, 0.0, 0.0, 1.0);
	return out;
}
