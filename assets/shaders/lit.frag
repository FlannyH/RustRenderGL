#version 420 core

// Vertex output / Fragment input
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

out vec4 frag_color;

void main() {
    frag_color = vec4(1.0, 1.0, 1.0, 1.0) * texture(tex_alb, o_uv0);
    //frag_color = vec4((o_normal + 1.0) / 2.0, 1);
}