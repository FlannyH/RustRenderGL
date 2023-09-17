#version 460

// Vertex input
layout (location = 0) in vec3 i_position;
layout (location = 1) in vec4 i_colour;

// Global constant buffer
layout (std140, binding = 0) uniform const_buffer
{
	uniform mat4 u_view_projection_matrix;
};

// Model specific data
uniform mat4 u_model_matrix;

// Vertex output / Fragment input
out vec4 o_colour;

void main()
{
	gl_Position = u_view_projection_matrix * /*u_model_matrix * */vec4(i_position, 1);
    o_colour = i_colour;
}