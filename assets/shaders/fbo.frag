#version 460

out vec4 frag_colour;
in vec2 texcoord;

uniform layout (binding = 0) sampler2D scene_colour;

void main()
{
	//Get scene colour
    vec4 colour = texture(scene_colour, texcoord);
	if (colour.a < 0.01f)
		discard;
	
	//Return color
	frag_colour = colour;
} 