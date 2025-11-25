// VERTEX_SHADER
#version 410 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoords;

out vec2 TexCoords;

void main()
{
    TexCoords   = aTexCoords;
    gl_Position = vec4(aPos, 1.0);
}

// FRAGMENT_SHADER
#version 410 core
in vec2 TexCoords;
out vec4 FragColor;

uniform sampler2D scene;

void main()
{
    // Size of one texel, in UV space
    vec2 texelSize = 1.0 / vec2(textureSize(scene, 0));

    // 3x3 Gaussian kernel, normalized by 1/16
    float kernel[9] = float[](
        1.0, 2.0, 1.0,
        2.0, 4.0, 2.0,
        1.0, 2.0, 1.0
    );

    vec3 result = vec3(0.0);
    float norm = 16.0;
    int idx = 0;

    for (int y = -1; y <= 1; ++y) {
        for (int x = -1; x <= 1; ++x) {
            vec2 offset = vec2(x, y) * texelSize;
            vec3 sampleCol = texture(scene, TexCoords + offset).rgb;
            result += sampleCol * kernel[idx];
            idx++;
        }
    }

    result /= norm;
    FragColor = vec4(result, 1.0);
}
