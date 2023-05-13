#version 330 core

// Vertex output / Fragment input
in vec4 o_colour;
in vec2 o_uv0;
in vec2 o_uv1;

out vec4 frag_color;

void main() {
    frag_color = vec4(1.0, 0.0, 1.0, 1.0);
}