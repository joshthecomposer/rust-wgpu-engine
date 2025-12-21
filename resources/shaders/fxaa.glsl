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
in vec2 TexCoords;
out vec4 FragColor;

uniform sampler2D uColor;
uniform vec2 uInvResolution; // 1.0/w, 1.0/h

float luma(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }

void main() {
    vec3 rgbM = texture(uColor, TexCoords).rgb;
    float lumaM = luma(rgbM);

    float lumaN = luma(texture(uColor, TexCoords + vec2(0.0,  uInvResolution.y)).rgb);
    float lumaS = luma(texture(uColor, TexCoords + vec2(0.0, -uInvResolution.y)).rgb);
    float lumaE = luma(texture(uColor, TexCoords + vec2( uInvResolution.x, 0.0)).rgb);
    float lumaW = luma(texture(uColor, TexCoords + vec2(-uInvResolution.x, 0.0)).rgb);

    float lumaMin = min(lumaM, min(min(lumaN,lumaS), min(lumaE,lumaW)));
    float lumaMax = max(lumaM, max(max(lumaN,lumaS), max(lumaE,lumaW)));
    float contrast = lumaMax - lumaMin;

    // If no edge, return original
    if (contrast < 0.05) {
        FragColor = vec4(rgbM, 1.0);
        return;
    }

    // Edge direction (gradient)
    vec2 dir = vec2(-(lumaN - lumaS), (lumaE - lumaW));
    float dirReduce = max((lumaN + lumaS + lumaE + lumaW) * 0.25 * 0.5, 1e-6);
    dir = clamp(dir / dirReduce, vec2(-8.0), vec2(8.0)) * uInvResolution;

    // Sample along the edge
    vec3 rgbA = 0.5 * (
        texture(uColor, TexCoords + dir * (1.0/3.0 - 0.5)).rgb +
        texture(uColor, TexCoords + dir * (2.0/3.0 - 0.5)).rgb
    );
    vec3 rgbB = rgbA * 0.5 + 0.25 * (
        texture(uColor, TexCoords + dir * -0.5).rgb +
        texture(uColor, TexCoords + dir *  0.5).rgb
    );

    float lumaB = luma(rgbB);
    // Choose less “out of range” result
    vec3 outRgb = (lumaB < lumaMin || lumaB > lumaMax) ? rgbA : rgbB;

    FragColor = vec4(outRgb, 1.0);

	//// DEBUG CHECK (red lines)
	//if (contrast < 0.05) {
	//	FragColor = vec4(0.0, 0.0, 0.0, 1.0); // non-edge
	//} else {
	//	FragColor = vec4(1.0, 0.0, 0.0, 1.0); // EDGE
	//}
}
