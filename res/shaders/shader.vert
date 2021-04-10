#version 450

layout (location = 0) in vec3 a_position;
layout (location = 1) in vec3 a_color;
layout (location = 2) in vec2 a_tex_coords;

layout (location = 0) out vec2 v_tex_coords;
layout (location = 1) out vec3 v_color;

layout (set = 0, binding = 0) uniform Uniforms {
    mat4 u_proj;
};

void main() {
    v_tex_coords = a_tex_coords;
    v_color = a_color;
    vec4 position = u_proj * vec4(a_position, 1.0);
    gl_Position = position;
}