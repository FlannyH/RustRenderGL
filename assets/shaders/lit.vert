#version 460

// Vertex input
layout (location = 0) in vec3 i_position;
layout (location = 1) in vec3 i_normal;
layout (location = 2) in vec4 i_tangent;
layout (location = 3) in vec4 i_colour;
layout (location = 4) in vec2 i_uv0;
layout (location = 5) in vec2 i_uv1;

// Global constant buffer
layout (std140) uniform const_buffer
{
	uniform mat4 u_view_projection_matrix;
};

// Model specific data
uniform mat4 u_model_matrix;

// Vertex output / Fragment input
out vec4 o_colour;
out vec2 o_uv0;
out vec2 o_uv1;

void main()
{
	gl_Position = /*u_view_projection_matrix * u_model_matrix * */vec4(i_position, 1);
    o_colour = i_colour;
    o_uv0 = i_uv0;
    o_uv1 = i_uv1;
}