// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec3 a_normal;
layout (location = 2) in vec2 a_tex_coords;
layout (location = 3) in ivec4 bone_ids;
layout (location = 4) in vec4 bone_weights;

out vec2 TexCoords;
out vec3 Normal;
out vec3 FragPos;
out vec4 FragPosLightSpace;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;
uniform mat4 light_space_mat;

// Animated model stuff
uniform bool is_animated;
const int MAX_BONE_INFLUENCE = 4;
const int MAX_BONES = 100;

uniform mat4 bone_transforms[MAX_BONES];



void main()
{
	if (is_animated) {
		vec4 totalPosition = vec4(0.0f);
		vec3 totalNormal = vec3(0.0f);
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


			mat3 boneNormalMatrix = transpose(inverse(mat3(bone_transforms[bone_ids[i]])));
			totalNormal += boneNormalMatrix * a_normal * bone_weights[i];
		}

		FragPos = vec3(model * totalPosition);
		FragPosLightSpace = light_space_mat * vec4(FragPos, 1.0);

		mat3 normalMatrix = transpose(inverse(mat3(model)));
		Normal = normalize(normalMatrix * totalNormal);

		mat4 viewModel = view * model;
		gl_Position =  projection * viewModel * totalPosition;
		TexCoords = a_tex_coords;
	} else {
		FragPos = vec3(model * vec4(a_pos, 1.0));
		Normal = mat3(transpose(inverse(model))) * a_normal;  
		TexCoords = a_tex_coords;    
		FragPosLightSpace = light_space_mat * vec4(FragPos, 1.0);
		gl_Position = projection * view * vec4(FragPos, 1.0);
	}
}

// FRAGMENT_SHADER
#version 460 core
out vec4 FragColor;

in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoords;
in vec4 FragPosLightSpace;

uniform bool has_opacity_texture;
uniform sampler2D shadow_map;

struct Material {
	sampler2D Diffuse;
	sampler2D Specular;
	sampler2D Emissive;
	sampler2D Opacity;
};
uniform Material material;

struct DirLight {
 	vec3 direction;
	vec3 view_pos;

	vec3 ambient;
	vec3 diffuse;
	vec3 specular;
};
uniform DirLight dir_light;
uniform float bias_scalar;
uniform vec3 view_position;
uniform bool alpha_test_pass;

float ShadowCalculation(float dot_light_normal) {
	vec3 pos = FragPosLightSpace.xyz * 0.5 + 0.5;
	// if (pos.z > 1.0) {
	// 	pos.z = 1.0;
	// }
	if (pos.x < 0.0 || pos.x > 1.0 ||
		pos.y < 0.0 || pos.y > 1.0 ||
		pos.z < 0.0 || pos.z > 1.0) {
		// treat outside the map as lit
		return 1.0;
	}

	float bias = max(bias_scalar * (1.0 - dot_light_normal), 0.0005);

	float shadow = 0.0;
	vec2 texel_size = 1.0 / textureSize(shadow_map, 0);
	for (int x = -1; x <= 1; ++x) {
		for (int y = -1; y <=1; ++y) {
			float depth = texture(shadow_map, pos.xy + vec2(x, y) * texel_size).r;
			shadow += (depth + bias) < pos.z ? 0.0 : 1.0;
		}
	}

	return shadow / 9.0; 

}

vec4 calculate_directional_light() {
    vec3 lightColor = dir_light.diffuse;
	vec3 tex_color = texture(material.Diffuse, TexCoords).rgb;
	vec3 spec_color = texture(material.Specular, TexCoords).rgb;
	vec3 emiss_color = texture(material.Emissive, TexCoords).rgb;
	
	float alpha = texture(material.Diffuse, TexCoords).a;

	if (alpha_test_pass && alpha < 0.1)
		discard;

	// Ambient
    vec3 ambient = vec3(dir_light.ambient);
	
	// Diffuse
    // vec3 lightDir = normalize(dir_light.view_pos - FragPos);
	vec3 lightDir = normalize(dir_light.direction);
    vec3 norm = normalize(Normal);
	float dot_light_normal = dot(lightDir, norm);
    float diff = max(dot_light_normal, 0.0);
    vec3 diffuse = diff * lightColor;

	// Specular
	vec3 viewDir = normalize(view_position - FragPos);
	vec3 reflectDir = reflect(-lightDir, norm);
	float spec = pow(max(dot(viewDir, reflectDir), 0.0), 36.0);
	vec3 specular = dir_light.specular * spec * spec_color;

	float shadow = ShadowCalculation(dot_light_normal);

    vec3 result_rgb = ((shadow * (diffuse + specular )) + ambient) * tex_color.rgb + emiss_color;

	return vec4(result_rgb, alpha);
}

void main() {    
	vec4 result = calculate_directional_light();
	FragColor = vec4(result);
}
