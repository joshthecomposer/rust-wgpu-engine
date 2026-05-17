struct CameraUniform {
	view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) normal: vec3<f32>,
	@location(2) uv: vec2<f32>,
	@location(3) bone_ids: vec4<i32>,
	@location(4) bone_weights: vec4<f32>,
}

struct InstanceInput {
	@location(5) mm0: vec4<f32>,
	@location(6) mm1: vec4<f32>,
	@location(7) mm2: vec4<f32>,
	@location(8) mm3: vec4<f32>,
}

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
	let model_matrix = mat4x4<f32>(
		instance.mm0,
		instance.mm1,
		instance.mm2,
		instance.mm3,
	);

	var out: VertexOutput;
	out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
	return out;
}

struct FragmentOut {
	@location(0) color: vec4<f32>,
	@location(1) bright: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOut {
	var out: FragmentOut;
	out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
	out.bright = vec4<f32>(0.0, 0.0, 0.0, 1.0);
	return out;
}
