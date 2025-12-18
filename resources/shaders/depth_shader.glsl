// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 a_pos;
layout (location = 4) in ivec4 bone_ids;
layout (location = 5) in vec4  bone_weights;

// mat4 instance model, locations 6..9
layout (location = 6) in mat4 i_model;

uniform mat4 light_space_mat;
uniform bool is_animated;

const int MAX_BONE_INFLUENCE = 4;
const int MAX_BONES = 100;

// Same binding you used in the main animated shader
layout(std430, binding = 0) readonly buffer BonesBuffer {
    mat4 bones[]; // length = instance_count * MAX_BONES
};

void main()
{
    vec4 local_pos = vec4(a_pos, 1.0);

    if (is_animated) {
        int base = gl_InstanceID * MAX_BONES;

        vec4 skinned = vec4(0.0);
        float wsum = 0.0;

        for (int i = 0; i < MAX_BONE_INFLUENCE; i++) {
            int bid = bone_ids[i];
            float w = bone_weights[i];
            if (bid < 0 || w <= 0.0) continue;
            if (bid >= MAX_BONES) { skinned = local_pos; wsum = 1.0; break; }

            skinned += (bones[base + bid] * local_pos) * w;
            wsum += w;
        }

        // safety: if no weights, fall back to bind pose position
        if (wsum <= 0.0001) skinned = local_pos;

        local_pos = skinned;
    }

    gl_Position = light_space_mat * i_model * local_pos;
}

// FRAGMENT_SHADER
#version 460 core

void main()
{             
    // gl_FragDepth = gl_FragCoord.z;
}
