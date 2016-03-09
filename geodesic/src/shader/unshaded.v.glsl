#version 150 core

uniform vec4 color;
uniform mat4 model;
uniform mat4 view;
uniform mat4 proj;

in vec3 position;

out vec4 v_color;

void main() {
    v_color = color;
    gl_Position = proj * view * model * vec4(position, 1.0);
}