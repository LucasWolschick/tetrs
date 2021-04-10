#version 450

layout (location = 0) in vec2 v_tex_coords;
layout (location = 1) in vec3 v_color;

layout (location = 0) out vec4 f_frag_color;

layout (set = 1, binding = 0) uniform texture2D t_diffuse;
layout (set = 1, binding = 1) uniform sampler s_diffuse;

void main() {
    vec4 tex_color = texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords);
    if (tex_color.a == 0.0) {
        discard;
    }
    f_frag_color = tex_color * vec4(v_color, 1.0);
}