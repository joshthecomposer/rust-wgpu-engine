// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec2 a_tex_coords;
layout (location = 2) in mat4 instance_matrix;
layout (location = 6) in float instance_alpha;

// uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

out vec2 tex_coords;
out float particle_alpha;

void main() {
    gl_Position = projection * view * instance_matrix * vec4(a_pos, 1.0);
	tex_coords = a_tex_coords;
	particle_alpha  = instance_alpha;
}

// FRAGMENT_SHADER
#version 460 core

in vec2 tex_coords;
in float particle_alpha;

uniform sampler2D texture1;
uniform bool has_tex;

out vec4 FragColor;

void main() {
	vec4 color = vec4(0.58, 0.1, 0.1, particle_alpha);

	if (has_tex) {
		color = texture(texture1, tex_coords);
	}

    FragColor = vec4(color.rgb, color.a * particle_alpha);
}
