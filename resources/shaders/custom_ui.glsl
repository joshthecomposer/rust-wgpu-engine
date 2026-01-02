// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec2 a_position;
layout (location = 1) in vec4 a_color;
layout (location = 2) in vec2 a_uv;

out vec4 v_color;
out vec2 v_uv;

uniform vec2 u_screen_size;

void main() {
    // convert from screen coordinates (0,0 top-left) to NDC (-1,-1 bottom-left)
    vec2 ndc = (a_position / u_screen_size) * 2.0 - 1.0;
    ndc.y = -ndc.y; // flip Y so 0 is at top
    
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_color = a_color;
    v_uv = a_uv;
}

// FRAGMENT_SHADER
#version 460 core
in vec4 v_color;
in vec2 v_uv;

out vec4 FragColor;

uniform sampler2D u_texture;
uniform bool u_is_alpha_mask;

void main() {
    vec4 tex_color = texture(u_texture, v_uv);
    if (u_is_alpha_mask) {
        // use red channel as alpha mask for text rendering
        FragColor = v_color * vec4(1.0, 1.0, 1.0, tex_color.r);
    } else {
        FragColor = v_color * tex_color;
    }
}


