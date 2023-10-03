#version 430

// Vertex input
layout (location = 0) in vec3 i_position;
layout (location = 1) in vec3 i_normal;
layout (location = 2) in vec4 i_tangent;
layout (location = 3) in vec4 i_colour;
layout (location = 4) in vec2 i_uv0;
layout (location = 5) in vec2 i_uv1;

// Global constant buffer
layout (std140, binding = 0) uniform const_buffer
{
	uniform mat4 u_view_projection_matrix;
};

// Model specific data
layout (location = 4) uniform mat4 u_model_matrix;

// Vertex output / Fragment input
out vec3 o_world_pos;
out vec4 o_colour;
out vec3 o_normal;
out vec3 o_tangent;
out vec3 o_bitangent;
out vec2 o_uv0;
out vec2 o_uv1;

void main()
{
    o_world_pos = i_position;
	gl_Position = u_view_projection_matrix * u_model_matrix * vec4(i_position, 1);
    o_colour = i_colour;
    o_normal = i_normal;
    o_tangent = i_tangent.xyz;
    o_bitangent = cross(i_normal, i_tangent.xyz) * i_tangent.w;
    o_uv0 = i_uv0;
    o_uv1 = i_uv1;
}