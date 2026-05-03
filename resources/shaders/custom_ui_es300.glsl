// VERTEX_SHADER
#version 300 es
precision highp float;
precision highp int;

layout (location = 0) in vec2 a_position;
layout (location = 1) in vec4 a_color;
layout (location = 2) in vec2 a_uv;
layout (location = 3) in vec4 a_rect_bounds;
layout (location = 4) in float a_border_radius;

out vec4 v_color;
out vec2 v_uv;
out vec4 v_rect_bounds;
out float v_border_radius;

uniform vec2 u_screen_size;

void main() {
    vec2 ndc = (a_position / u_screen_size) * 2.0 - 1.0;
    ndc.y = -ndc.y;

    gl_Position = vec4(ndc, 0.0, 1.0);
    v_color = a_color;
    v_uv = a_uv;
    v_rect_bounds = a_rect_bounds;
    v_border_radius = a_border_radius;
}

// FRAGMENT_SHADER
#version 300 es
precision highp float;
precision highp int;
precision highp sampler2D;

in vec4 v_color;
in vec2 v_uv;
in vec4 v_rect_bounds;
in float v_border_radius;

out vec4 FragColor;

uniform sampler2D u_texture;
uniform bool u_is_alpha_mask;
uniform vec2 u_screen_size;

void main() {
    if (v_border_radius > 0.0) {
        vec2 pixel_pos = vec2(gl_FragCoord.x, u_screen_size.y - gl_FragCoord.y);
        vec2 rect_center = v_rect_bounds.xy + v_rect_bounds.zw * 0.5;
        vec2 half_size = v_rect_bounds.zw * 0.5;

        vec2 p = pixel_pos - rect_center;
        vec2 d = abs(p) - (half_size - vec2(v_border_radius));
        float dist = length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - v_border_radius;

        if (dist > 0.0) {
            discard;
        }
    }

    vec4 tex_color = texture(u_texture, v_uv);
    if (u_is_alpha_mask) {
        FragColor = v_color * vec4(1.0, 1.0, 1.0, tex_color.r);
    } else {
        FragColor = v_color * tex_color;
    }
}
