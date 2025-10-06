// VERTEX_SHADER
#version 410 core
layout (location = 0) in vec3 a_pos;
layout (location = 4) in ivec4 bone_ids;
layout (location = 5) in vec4 bone_weights;

uniform mat4 light_space_mat;
uniform mat4 model;
uniform bool is_animated;

const int MAX_BONE_INFLUENCE = 4;
const int MAX_BONES = 100;

uniform mat4 bone_transforms[MAX_BONES];

void main()
{
	if (is_animated) {
		vec4 totalPosition = vec4(0.0f);
		for(int i = 0 ; i < MAX_BONE_INFLUENCE; i++)
		{
			if(bone_ids[i] == -1) 
				continue;
			if(bone_ids[i] >=MAX_BONES) 
			{
				totalPosition = vec4(a_pos,1.0f);
				break;
			}
			vec4 localPosition = bone_transforms[bone_ids[i]] * vec4(a_pos,1.0f);
			totalPosition += localPosition * bone_weights[i];
		}
		gl_Position = light_space_mat * model * totalPosition;
	} else {
		gl_Position = light_space_mat * model * vec4(a_pos, 1.0);
	}
}

// FRAGMENT_SHADER
#version 410 core

void main()
{             
    // gl_FragDepth = gl_FragCoord.z;
}
