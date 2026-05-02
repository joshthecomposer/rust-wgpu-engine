// VERTEX_SHADER
#version 300 es
precision highp float;

layout(location = 0) in vec3 aPos;
layout(location = 1) in vec2 aTexCoords;

out vec2 TexCoords;

void main()
{
    TexCoords = aTexCoords;
    gl_Position = vec4(aPos, 1.0);
}

// FRAGMENT_SHADER
#version 300 es
precision highp float;

in vec2 TexCoords;

layout(location = 0) out vec4 FragColor;

uniform sampler2D hdrBuffer;
uniform sampler2D bloomBuffer;
uniform sampler2D uDepth;

uniform bool hdr;
uniform float exposure;
uniform float bloomStrength;

uniform mat4 uInvProj;

const vec3  fogColor = vec3(0.05, 0.05, 0.06);
const float fogStart = 10.0;
const float fogEnd = 85.0;
const float fogStrength = 0.95;

float viewZFromDepth(float depth01, vec2 uv)
{
    float z = depth01 * 2.0 - 1.0;
    vec4 ndc = vec4(uv * 2.0 - 1.0, z, 1.0);
    vec4 view = uInvProj * ndc;
    view.xyz /= view.w;
    return -view.z;
}

float fogFactorLinear(float dist)
{
    float f = (dist - fogStart) / max(fogEnd - fogStart, 1e-6);
    return clamp(f, 0.0, 1.0) * fogStrength;
}

void main()
{
    const float gamma = 2.2;

    vec3 hdrColor = texture(hdrBuffer, TexCoords).rgb;
    vec3 bloom    = texture(bloomBuffer, TexCoords).rgb;

    vec3 color = hdrColor + bloom * bloomStrength;

    float depth01 = texture(uDepth, TexCoords).r;

    float dist = viewZFromDepth(depth01, TexCoords);
    float fogF = fogFactorLinear(dist);

    if (depth01 >= 0.999999) {
        fogF = 0.85;
    }

    color = mix(color, fogColor, fogF);

    if (hdr)
    {
        vec3 result = vec3(1.0) - exp(-color * exposure);
        result = pow(result, vec3(1.0 / gamma));
        FragColor = vec4(result, 1.0);
    }
    else
    {
        vec3 result = pow(color, vec3(1.0 / gamma));
        FragColor = vec4(result, 1.0);
    }
}
