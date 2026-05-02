// VERTEX_SHADER
#version 300 es
precision highp float;

layout (location = 0) in vec2 a_position;
layout (location = 1) in vec3 a_color;

out vec3 v_color;

void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
    v_color = a_color;
}

// FRAGMENT_SHADER
#version 300 es
precision highp float;

in vec3 v_color;
out vec4 FragColor;

void main() {
    FragColor = vec4(v_color, 1.0);
}
