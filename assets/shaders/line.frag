#version 420 core

// Vertex output / Fragment input
in vec4 o_colour;
out vec4 frag_color;

void main() {
    frag_color = o_colour;
}