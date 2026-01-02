// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec2 a_position;
layout (location = 1) in vec4 a_color;

out vec4 v_color;

uniform vec2 u_screen_size;

void main() {
    // convert from screen coordinates (0,0 top-left) to NDC (-1,-1 bottom-left)
    vec2 ndc = (a_position / u_screen_size) * 2.0 - 1.0;
    ndc.y = -ndc.y; // flip Y so 0 is at top
    
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_color = a_color;
}

// FRAGMENT_SHADER
#version 460 core
in vec4 v_color;

out vec4 FragColor;

void main() {
    FragColor = v_color;
}

