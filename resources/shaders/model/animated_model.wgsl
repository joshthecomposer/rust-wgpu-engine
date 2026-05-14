const MAX_BONES: u32 = 200u;

struct CameraUniform {
	view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct BoneUniforms {
	matrices: array<mat4x4<f32>, MAX_BONES>,
}
@group(2) @binding(0)
var<uniform> bones: BoneUniforms;


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
}

struct InstanceInput {
	@location(5) mm0: vec4<f32>,
	@location(6) mm1: vec4<f32>,
	@location(7) mm2: vec4<f32>,
	@location(8) mm3: vec4<f32>,
}

fn skin_position(model: VertexInput) -> vec4<f32> {
	var sum = vec4<f32>(0.0, 0.0, 0.0, 0.0);
	var any_bone = false;

	for (var i: u32 = 0u; i < 4u; i = i + 1u) {
		let w = model.bone_weights[i];
		if (w <= 0.0) {
			continue;
		}

		let bid = model.bone_ids[i];
		if (bid < 0) {
			continue;
		}

		let bi = u32(bid);

		if (bi >= MAX_BONES) {
			return vec4<f32>(model.position, 1.0);
		}

		let bone_m = bones.matrices[bi];
		sum = sum + bone_m * vec4<f32>(model.position, 1.0) * w;

		any_bone = true;
	}

	if (!any_bone) {
		return vec4<f32>(model.position, 1.0);
	}

	return sum;
}

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput ) -> VertexOutput {
	var out: VertexOutput;

	let model_matrix = mat4x4<f32>(
		instance.mm0,
		instance.mm1,
		instance.mm2,
		instance.mm3,
	);

	let skinned_local = skin_position(model);
	out.clip_position = camera.view_proj * model_matrix * skinned_local;

	out.uv = model.uv;
	return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return textureSample(t_diffuse, s_diffuse, in.uv);
}
