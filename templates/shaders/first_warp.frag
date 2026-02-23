#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform Params {
    float tint_r;
    float tint_g;
    float tint_b;
    float brightness;
    float warp_amount;
} params;

void main() {
    vec2 warped = fragTexCoord;
    warped.x += params.warp_amount * 0.3;
    vec4 tex = texture(texSampler, warped);
    vec3 tint = vec3(params.tint_r, params.tint_g, params.tint_b);
    outColor = vec4(tex.rgb * tint * params.brightness, tex.a);
}
