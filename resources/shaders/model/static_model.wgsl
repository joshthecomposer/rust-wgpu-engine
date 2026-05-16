@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

struct CameraUniform {
	view_proj: mat4x4<f32>,
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
@group(2) @binding(0)
var<uniform> dir_light: DirLightUniform;

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
	out.uv = model.uv;

	return out;
}

struct FragmentOut {
	@location(0) color: vec4<f32>,
	@location(1) bright: vec4<f32>,
}


fn calculate_directional_light(in: VertexOutput) -> vec4<f32> {
	let light_color = dir_light.diffuse;
	let view_position = dir_light.view_pos;
	
	let view_dist = length(view_position - in.clip_position);
	let lod = clamp((view_dist - 5.0) / 5.0, 0.0, 30.0);
	// equivalent to textureLod() in opengl
	let tex_sample = textureSampleLevel(t_diffuse, s_diffuse, in.uv, lod);
	
	if (tex_sample.a < 0.5) {
		discard;
	}

	// specular
	// emissive

	let flat_ambient = dir_light.ambient;
	let light_dir = normalize(dir_light.direction);
	let norm = normalize(in.normal);

	let dot_light_normal = dot(light_dir, norm);
	let diff = max(dot_light_normal, 0.0);
	let diffuse = diff * light_color;

	// do shadows here

	return vec4<f32>((flat_ambient.rgb + diffuse.rgb) * tex_sample.rgb, tex_sample.a);

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
	let lit = calculate_directional_light(in);

	var out: FragmentOut;

	out.color = lit;

	out.bright = vec4<f32>(extract_bright(lit.rgb, 1.0, 0.5), lit.a);
	return out;
}
