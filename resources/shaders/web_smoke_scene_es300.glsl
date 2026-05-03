// VERTEX_SHADER
#version 300 es
precision highp float;

layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec3 a_normal;
layout (location = 2) in vec2 a_uv;
layout (location = 3) in vec4 a_base_color;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;
uniform float elapsed;

out vec3 v_normal;
out vec2 v_uv;
out vec4 v_base_color;

void main() {
    float breathe = sin(elapsed * 1.7) * 0.035;
    vec3 position = a_pos + a_normal * breathe;
    mat3 normal_matrix = mat3(model);

    gl_Position = projection * view * model * vec4(position, 1.0);
    v_normal = normalize(normal_matrix * a_normal);
    v_uv = a_uv;
    v_base_color = a_base_color;
}

// FRAGMENT_SHADER
#version 300 es
precision highp float;

in vec3 v_normal;
in vec2 v_uv;
in vec4 v_base_color;
uniform sampler2D diffuse_texture;
uniform bool use_texture;
out vec4 FragColor;

void main() {
    vec3 light_dir = normalize(vec3(0.45, 0.75, 0.35));
    float diffuse = max(dot(normalize(v_normal), light_dir), 0.0);
    vec4 tex_color = use_texture ? texture(diffuse_texture, v_uv) : v_base_color;
    vec4 base_color = tex_color * v_base_color;
    vec3 lit_color = base_color.rgb * (0.28 + diffuse * 0.72);

    FragColor = vec4(lit_color, base_color.a);
}
