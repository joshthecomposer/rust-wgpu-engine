// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec2 a_tex_coords;
layout (location = 2) in mat4 instance_matrix;
layout (location = 6) in float instance_alpha;
layout (location = 7) in vec4 instance_color;

// uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

out vec2 tex_coords;
out float particle_alpha;
out vec4 particle_color;

void main() {
    gl_Position = projection * view * instance_matrix * vec4(a_pos, 1.0);
	tex_coords = a_tex_coords;
	particle_alpha  = instance_alpha;
	particle_color = instance_color;
}

// FRAGMENT_SHADER
#version 460 core
layout (location = 0) out vec4 FragColor;
layout (location = 1) out vec4 BrightColor;

in vec2 tex_coords;
in float particle_alpha;
in vec4 particle_color;

uniform sampler2D texture1;
uniform bool has_tex;
uniform bool texture_has_alpha;

//out vec4 FragColor;

vec4 luminance_texture() {
    if (particle_alpha <= 0.001) discard;

    vec4 tex = has_tex ? texture(texture1, tex_coords) : vec4(1.0);

    float brightness = dot(tex.rgb, vec3(0.299, 0.587, 0.114)); // or max(tex.r, max(tex.g, tex.b));
    float mask = has_tex ? brightness : 1.0;

    float outA = particle_color.a * particle_alpha * mask;

    if (outA <= 0.001) discard;

    vec3 outRGB = particle_color.rgb;

    return vec4(outRGB.rgb, outA);
}

vec4 alpha_texture() {
	if (particle_color.a <= 0.001 || particle_alpha <= 0.001) discard;

	float mask = has_tex ? texture(texture1, tex_coords).a : 1.0;

	float outA  = particle_color.a * particle_alpha * mask;
	vec3  outRGB = particle_color.rgb;  // pure tint color

	if (outA <= 0.001) discard;

	return vec4(outRGB.rgb, outA);
}

void main() {
	vec4 color;

	if (texture_has_alpha) {
		color = alpha_texture();
	} else {
		color = luminance_texture();
	}

	FragColor = color;
}


