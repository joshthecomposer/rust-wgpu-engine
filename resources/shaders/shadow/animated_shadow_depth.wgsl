const MAX_BONES: u32 = 200u;

struct LightSpaceUniform {
    light_view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> light: LightSpaceUniform;

@group(1) @binding(0)
var<uniform> bone_matrices: array<mat4x4<f32>, MAX_BONES>;

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

struct SkinnedVertex {
    position: vec4<f32>,
}

fn skin_vertex(model: VertexInput) -> SkinnedVertex {
    var sum_pos = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let bid = model.bone_ids[i];
        if (bid < 0) {
            continue;
        }
        let bi = u32(bid);
        if (bi >= MAX_BONES) {
            return SkinnedVertex(vec4<f32>(model.position, 1.0));
        }
        let w = model.bone_weights[i];
        if (w <= 0.0) {
            continue;
        }
        let bone_m = bone_matrices[bi];
        sum_pos = sum_pos + (bone_m * vec4<f32>(model.position, 1.0)) * w;
    }

    let sum_len_sq = dot(sum_pos.xyz, sum_pos.xyz) + sum_pos.w * sum_pos.w;
    if (sum_len_sq <= 1e-20) {
        return SkinnedVertex(vec4<f32>(model.position, 1.0));
    }
    return SkinnedVertex(sum_pos);
}

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> @builtin(position) vec4<f32> {
    let model_matrix = mat4x4<f32>(
        instance.mm0,
        instance.mm1,
        instance.mm2,
        instance.mm3,
    );
    let skinned = skin_vertex(model);
    let world = model_matrix * skinned.position;
    return light.light_view_proj * world;
}
