#version 460
in layout (location = 0) vec2 a_position;
in layout (location = 1) vec2 a_texcoord;
out vec2 texcoord;

void main()
{
    gl_Position = vec4(a_position, 0, 1);
	texcoord = a_texcoord;
}