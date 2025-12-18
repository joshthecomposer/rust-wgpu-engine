// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec2 a_tex_coords;

out vec2 TexCoords;

uniform vec2 u_offset;
uniform vec2 u_scale;
uniform bool u_flip_v;  // flip V coordinate for FBO textures

void main()
{
    vec2 uv = a_tex_coords;
    if (u_flip_v) {
        uv.y = 1.0 - uv.y;
    }
    TexCoords = uv;

    vec2 pos = a_pos * u_scale + u_offset;

    gl_Position = vec4(pos, 0.0, 1.0);
}

// FRAGMENT_SHADER
#version 460 core
out vec4 FragColor;

in vec2 TexCoords;

uniform sampler2D ui_texture;

void main()
{    
    FragColor = texture(ui_texture, TexCoords);
}