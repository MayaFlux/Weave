#version 450

layout(push_constant) uniform Params {
    float tint_r;
    float tint_g;
    float tint_b;
    float brightness;
};

layout(binding = 0) uniform sampler2D texSampler;
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 out_color;

void main() {
    vec4 tex = texture(texSampler, uv);
    vec3 tint = vec3(tint_r, tint_g, tint_b);
    out_color = vec4(tex.rgb * tint * brightness, tex.a);
}
