// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoords;
out vec2 uv;
void main()
{
    uv = aTexCoords;
    gl_Position = vec4(aPos, 1.0);
}

// FRAGMENT_SHADER
#version 460 core
uniform sampler2D src;
uniform vec2 texelSize;

in vec2 uv;
out vec4 outColor;

void main() {
    vec3 c = vec3(0.0);

    c += texture(src, uv + texelSize * vec2(-1, 0)).rgb;
    c += texture(src, uv + texelSize * vec2( 1, 0)).rgb;
    c += texture(src, uv + texelSize * vec2( 0,-1)).rgb;
    c += texture(src, uv + texelSize * vec2( 0, 1)).rgb;
    c += texture(src, uv).rgb * 2.0;

    c /= 6.0;
    outColor = vec4(c, 1.0);
}
