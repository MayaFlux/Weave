#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    float time;
    float feedback_amount;
    float zoom;
} params;

void main() {
    vec2 uv = fragTexCoord;

    vec2 sample_offset = texture(texSampler, uv).rg - 0.5;

    uv += sample_offset * params.feedback_amount;

    vec2 center = vec2(0.5, 0.5);
    uv = (uv - center) * params.zoom + center;

    outColor = texture(texSampler, uv);
}
