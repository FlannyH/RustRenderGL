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
    0, 32, 8, 40, 2, 34, 10, 42,
    48, 16, 56, 24, 50, 18, 58, 26,
    12, 44, 4, 36, 14, 46, 6, 38,
    60, 28, 52, 20, 62, 30, 54, 22,
    3, 35, 11, 43, 1, 33, 9, 41,
    51, 19, 59, 27, 49, 17, 57, 25,
    15, 47, 7, 39, 13, 45, 5, 37,
    63, 31, 55, 23, 61, 29, 53, 21,
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

    // Sample point lights
    for (uint i = 0; i < n_lights; ++i) {
        vec3 surface_to_light = lights[i].position.xyz - o_world_pos;
        float distance = length(surface_to_light);
        float n_dot_l = dot(o_normal, surface_to_light) / distance;
        float attenuation = 1 / (distance * distance);
        light_acc += n_dot_l * lights[i].color * lights[i].intensity * attenuation;
    }
    
    // Get albedo color from texture, or make it white if it doesn't have a texture
    vec4 albedo = (use_tex_alb != 0) ? (texture(tex_alb, o_uv0)) : vec4(1.0, 1.0, 1.0, 1.0);

    // Quantize and dither
    float color_depth = 255.0;
    int x_mod_8 = int(mod(gl_FragCoord.x, 8.0));
    int y_mod_8 = int(mod(gl_FragCoord.y, 8.0));
    int index = x_mod_8 + (y_mod_8 * 8);
    float dither = dither_table[index] / 64.0;
    dither /= color_depth;
    frag_color = vec4(light_acc, 1.0) * albedo;
    frag_color += vec4(dither);
    frag_color *= vec4(color_depth);
    frag_color = floor(frag_color);
    frag_color /= vec4(color_depth);
}