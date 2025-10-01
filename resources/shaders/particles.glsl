// VERTEX_SHADER
#version 460 core
layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec2 a_tex_coords;
layout (location = 2) in mat4 instance_matrix;
layout (location = 6) in float instance_alpha;
layout (location = 7) in vec4 instance_color;

// uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

out vec2 tex_coords;
out float particle_alpha;
out vec4 particle_color;

void main() {
    gl_Position = projection * view * instance_matrix * vec4(a_pos, 1.0);
	tex_coords = a_tex_coords;
	particle_alpha  = instance_alpha;
	particle_color = instance_color;
}

// FRAGMENT_SHADER
#version 460 core

in vec2 tex_coords;
in float particle_alpha;
in vec4 particle_color;

uniform sampler2D texture1;
uniform bool has_tex;

out vec4 FragColor;

// void main() {
// 	vec4 color = particle_color;
// 
// 	if (has_tex) {
// 		color = texture(texture1, tex_coords);
//     	FragColor = vec4(color.rgb, color.a * particle_alpha);
// 	} else {
//     	FragColor = color;
// 	}
// 
// }

// ALMOST WORKED
// void main() {
//     if (particle_color.a <= 0.001) discard;
// 
//     float mask = has_tex ? texture(texture1, tex_coords).a : 1.0;
//     float a = particle_alpha * particle_color.a * mask;
//     vec3 rgb = particle_color.rgb * a;
// 
//     if (a <= 0.001) discard;     // helps kill any remaining halo
//     FragColor = vec4(rgb, a);
// }

void main() {
    if (particle_color.a <= 0.001 || particle_alpha <= 0.001) discard;

    float mask = has_tex ? texture(texture1, tex_coords).a : 1.0;

    float outA  = particle_color.a * particle_alpha * mask;
    vec3  outRGB = particle_color.rgb;  // pure tint color

    if (outA <= 0.001) discard;
    FragColor = vec4(outRGB, outA);
}
