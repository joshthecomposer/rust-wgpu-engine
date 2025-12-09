// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec2 a_tex_coords;

out vec2 TexCoords;

void main()
{
    TexCoords = a_tex_coords;
    gl_Position = vec4(a_pos, 0.0, 1.0);
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

