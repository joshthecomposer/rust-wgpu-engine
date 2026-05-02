// VERTEX_SHADER
#version 300 es
precision highp float;
precision highp int;

layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec3 a_normal;
layout (location = 2) in vec2 a_tex_coords;
layout (location = 3) in vec4 a_base_color;

out vec2 TexCoords;
out vec3 Normal;
out vec3 FragPos;
out vec4 FragPosLightSpace;
out vec4 base_color;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;
uniform mat4 light_space_mat;

void main()
{
    FragPos = vec3(model * vec4(a_pos, 1.0));
    Normal = mat3(transpose(inverse(model))) * a_normal;
    TexCoords = a_tex_coords;
    FragPosLightSpace = light_space_mat * vec4(FragPos, 1.0);
    gl_Position = projection * view * vec4(FragPos, 1.0);
    base_color = a_base_color;
}

// FRAGMENT_SHADER
#version 300 es
precision highp float;
precision highp int;

in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoords;
in vec4 base_color;
in vec4 FragPosLightSpace;

uniform bool has_opacity_texture;
uniform sampler2D shadow_map;
uniform bool shadow_border_fallback;
uniform bool use_shadows;
uniform bool use_base_color;

struct Material {
    sampler2D Diffuse;
    sampler2D Specular;
    sampler2D Emissive;
    sampler2D Opacity;
};
uniform Material material;

struct DirLight {
    vec3 direction;
    vec3 view_pos;
    vec3 ambient;
    vec3 diffuse;
    vec3 specular;
};
uniform DirLight dir_light;

uniform samplerCube skybox;
uniform float bias_scalar;
uniform vec3 view_position;
uniform bool alpha_test_pass;
uniform bool selection_fresnel;
uniform bool do_reg_fresnel;
uniform float elapsed;
uniform bool flash_white;

out vec4 FragColor;

float fresnel_bias = 0.1;
float fresnel_scale = 1.0;
float fresnel_power = 3.0;

float ShadowCalculation(float dot_light_normal) {
    vec3 pos = FragPosLightSpace.xyz * 0.5 + 0.5;
    if (pos.x < 0.0 || pos.x > 1.0 ||
        pos.y < 0.0 || pos.y > 1.0 ||
        pos.z < 0.0 || pos.z > 1.0) {
        return 1.0;
    }

    float bias = max(bias_scalar * (1.0 - dot_light_normal), 0.0005);
    float shadow = 0.0;
    vec2 texel_size = 1.0 / vec2(textureSize(shadow_map, 0));

    for (int x = -1; x <= 1; ++x) {
        for (int y = -1; y <= 1; ++y) {
            vec2 sample_uv = pos.xy + vec2(x, y) * texel_size;
            if (shadow_border_fallback &&
                (sample_uv.x < 0.0 || sample_uv.x > 1.0 ||
                 sample_uv.y < 0.0 || sample_uv.y > 1.0)) {
                shadow += 1.0;
            } else {
                float depth = texture(shadow_map, sample_uv).r;
                shadow += (depth + bias) < pos.z ? 0.0 : 1.0;
            }
        }
    }
    return shadow / 9.0;
}

vec4 calculate_directional_light() {
    vec3 lightColor = dir_light.diffuse;
    vec3 tex_color;
    float alpha;

    if (use_base_color) {
        tex_color = base_color.rgb;
        alpha = base_color.a;
    } else {
        float view_dist = length(view_position - FragPos);
        float lod = clamp((view_dist - 5.0) / 5.0, 0.0, 30.0);
        vec4 tex_sample = textureLod(material.Diffuse, TexCoords, lod);
        vec3 safe_color = mix(vec3(1.0), tex_sample.rgb, tex_sample.a);
        tex_color = safe_color;
        alpha = tex_sample.a;
    }

    vec3 spec_color = texture(material.Specular, TexCoords).rgb;
    vec3 emiss_color = texture(material.Emissive, TexCoords).rgb;

    if (flash_white) {
        float t = mod(elapsed, 0.15);
        if (t < 0.075) {
            return vec4(1.0, 1.0, 1.0, alpha);
        } else {
            discard;
        }
    }

    vec3 flat_ambient = vec3(dir_light.ambient);
    vec3 envAmbient = texture(skybox, normalize(Normal)).rgb;
    vec3 ambient = max(mix(flat_ambient, envAmbient, 0.05), vec3(0.28));
    vec3 lightDir = normalize(dir_light.direction);
    vec3 norm = normalize(Normal);
    float dot_light_normal = dot(lightDir, norm);
    float diff = max(dot_light_normal, 0.0);
    vec3 diffuse = diff * lightColor;

    vec3 viewDir = normalize(view_position - FragPos);
    vec3 reflectDir = reflect(-lightDir, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), 36.0);
    vec3 specular = dir_light.specular * spec * spec_color;
    float shadow = use_shadows ? ShadowCalculation(dot_light_normal) : 1.0;

    vec3 result_rgb = ((shadow * (diffuse + specular)) + ambient) * tex_color + emiss_color;

    if (do_reg_fresnel) {
        float reg_fresnel = fresnel_bias + fresnel_scale * pow(1.0 - max(dot(norm, viewDir), 0.0), fresnel_power);
        vec3 reg_fresnel_color = vec3(1.0);
        result_rgb = mix(result_rgb, reg_fresnel_color, reg_fresnel * 0.6);
    }

    if (selection_fresnel) {
        float fresnel = fresnel_bias + fresnel_scale * pow(1.0 - max(dot(norm, viewDir), 0.0), fresnel_power);
        float pulse = 0.875 + 0.125 * sin(elapsed * 6.0);
        pulse = pow(pulse, 3.0);
        vec3 fresnel_color = vec3(2.0, 0.0, 3.5);
        vec3 glow = fresnel_color * pulse;
        result_rgb += fresnel * glow;
    }

    return vec4(result_rgb, alpha);
}

void main() {
    FragColor = calculate_directional_light();
}
