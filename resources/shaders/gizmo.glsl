// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 a_pos;

out vec3 FragPos;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;

void main()
{
	FragPos = vec3(model * vec4(a_pos, 1.0));
	gl_Position = projection * view * vec4(FragPos, 1.0);
}

// FRAGMENT_SHADER
#version 460 core
out vec4 FragColor;

in vec3 FragPos;

void main() {
	FragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
