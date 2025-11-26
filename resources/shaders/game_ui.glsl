// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec4 a_color;

out vec4 color;
out vec2 tex_coord;

void main() {
    gl_Position = vec4(aPos, 1.0);
    color = a_color;
	tex_coord = uv;
}

// FRAGMENT_SHADER
#version 460 core
out vec4 FragColor;

in vec4 color;
in vec2 tex_coord;

uniform sampler2D tex;
uniform bool use_texture;

void main() {
	if (use_texture) {
		vec4 tex_color = texture(tex, tex_coord);
		FragColor = mix(tex_color, color, 0.35);
	} else {
    	FragColor = vec4(color);
	}
}
