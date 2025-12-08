// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoords;

out vec2 TexCoords;

void main()
{
    TexCoords = aTexCoords;
    gl_Position = vec4(aPos, 1.0);
}

// FRAGMENT_SHADER
#version 460 core
out vec4 FragColor;

in vec2 TexCoords;

uniform sampler2D hdrBuffer;
uniform sampler2D bloomBuffer;
uniform bool hdr;
uniform float exposure;
uniform float bloomStrength;

void main()
{             
    const float gamma = 2.2;

    vec3 hdrColor = texture(hdrBuffer, TexCoords).rgb;
	vec3 bloom    = texture(bloomBuffer, TexCoords).rgb;

	vec3 color = hdrColor + bloom * bloomStrength;

    if(hdr)
    {
        vec3 result = vec3(1.0) - exp(-color * exposure);
        result = pow(result, vec3(1.0 / gamma));
        FragColor = vec4(result, 1.0);
    }
    else
    {
        vec3 result = pow(hdrColor, vec3(1.0 / gamma));
        FragColor = vec4(result, 1.0);
    }
}
