// VERTEX_SHADER
#version 300 es
precision highp float;

layout (location = 0) in vec3 aPos;

out vec3 texCoords;

uniform mat4 projection;
uniform mat4 view;

void main()
{
    vec4 pos = projection * view * vec4(aPos, 1.0);
    gl_Position = vec4(pos.x, pos.y, pos.w, pos.w);
    texCoords = vec3(aPos.x, aPos.y, -aPos.z);
}

// FRAGMENT_SHADER
#version 300 es
precision highp float;

in vec3 texCoords;

uniform samplerCube skybox;

uniform bool hdr_render_target;

out vec4 FragColor;

void main()
{
    vec3 c = texture(skybox, texCoords).rgb;
    if (!hdr_render_target) {
        c = pow(max(c, vec3(0.0)), vec3(1.0 / 2.2));
    }
    FragColor = vec4(c, 1.0);
}
