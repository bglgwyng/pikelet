#version 150 core

uniform mat4 proj;
uniform mat4 model;

in vec2 position;
in vec2 tex_coords;
out vec2 v_tex_coords;

void main() {
    gl_Position = proj * model * vec4(position, 0.0, 1.0);
    v_tex_coords = tex_coords;
}