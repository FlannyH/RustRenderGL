#version 430 core

struct Light{
    vec3 position;
    vec3 color;
    float intensity;
};

// Vertex output / Fragment input
in vec3 o_world_pos;
in vec4 o_colour;
in vec3 o_normal;
in vec3 o_tangent;
in vec3 o_bitangent;
in vec2 o_uv0;
in vec2 o_uv1;

layout (binding = 0) uniform sampler2D tex_alb;
layout (binding = 1) uniform sampler2D tex_nrm;
layout (binding = 2) uniform sampler2D tex_mtl_rgh;
layout (binding = 3) uniform sampler2D tex_emm;
layout (location = 0) uniform int use_tex_alb;
layout (location = 1) uniform int use_tex_nrm;
layout (location = 2) uniform int use_tex_mtl_rgh;
layout (location = 3) uniform int use_tex_emm;
layout (std140, binding = 0) buffer light
{
	uint n_lights;
    Light lights[];
};

out vec4 frag_color;

void main() {
    vec3 light_acc = vec3(0.0, 0.0, 0.0);
    for (uint i = 0; i < n_lights; ++i) {
        //vec3 surface_to_light = normalize(lights[i].position - o_world_pos);
        //float n_dot_l = dot(o_normal, surface_to_light);
        //light_acc += n_dot_l * lights[i].color * lights[i].intensity;
    }
    
    vec4 albedo = (use_tex_alb != 0) ? (texture(tex_alb, o_uv0)) : vec4(1.0, 1.0, 1.0, 1.0);

    frag_color = vec4(light_acc, 1.0) * albedo;
}