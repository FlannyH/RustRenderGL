#version 430 core

struct Light{
    vec4 position; // ignore w
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

const float dither_table[] = {
    0,  8,  2,  10,
    12, 4,  14, 6, 
    3,  11, 1,  9, 
    15, 7,  13, 5
};

layout (binding = 0) uniform sampler2D tex_alb;
layout (binding = 1) uniform sampler2D tex_nrm;
layout (binding = 2) uniform sampler2D tex_mtl_rgh;
layout (binding = 3) uniform sampler2D tex_emm;
layout (location = 0) uniform int use_tex_alb;
layout (location = 1) uniform int use_tex_nrm;
layout (location = 2) uniform int use_tex_mtl_rgh;
layout (location = 3) uniform int use_tex_emm;
layout (location = 4) uniform int n_lights;
layout (std430, binding = 0) buffer light
{
    Light lights[];
};

out vec4 frag_color;

void main() {
    vec3 light_acc = vec3(0.0, 0.0, 0.0);
    for (uint i = 0; i < n_lights; ++i) {
        vec3 surface_to_light = lights[i].position.xyz - o_world_pos;
        float distance = length(surface_to_light);
        float n_dot_l = dot(o_normal, surface_to_light) / distance;
        float attenuation = 1 / (distance * distance);
        light_acc += n_dot_l * lights[i].color * lights[i].intensity * attenuation;
    }
    
    vec4 albedo = (use_tex_alb != 0) ? (texture(tex_alb, o_uv0)) : vec4(1.0, 1.0, 1.0, 1.0);

    float color_depth = 256.0;
    int x_mod_4 = int(mod(gl_FragCoord.x, 4.0));
    int y_mod_4 = int(mod(gl_FragCoord.y, 4.0));
    int index = x_mod_4 + (y_mod_4 * 4);
    float dither = dither_table[index] / 16.0;
    dither /= color_depth;
    frag_color = vec4(light_acc, 1.0) * albedo;
    frag_color += vec4(dither);
    frag_color *= vec4(color_depth);
    frag_color = floor(frag_color);
    frag_color /= vec4(color_depth);
}