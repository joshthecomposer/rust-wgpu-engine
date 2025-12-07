// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;

out vec2 TexCoord;

void main() {
    gl_Position = vec4(aPos, 1.0);
    TexCoord = aTexCoord;
}

// FRAGMENT_SHADER
#version 460 core
out vec4 FragColor;

in vec2 TexCoord;

uniform sampler2D uTexture;

void main() {
    // Ultralight renders as premultiplied BGRA, we need to convert
    vec4 texColor = texture(uTexture, TexCoord);
    // BGRA -> RGBA swap (already handled by GL_BGRA on upload, but alpha is premultiplied)
    // For premultiplied alpha: color = texColor.rgb, alpha = texColor.a
    FragColor = texColor;
}

