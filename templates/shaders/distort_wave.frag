#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    float time;
    float frequency;
    float amplitude;
} params;

void main() {
    vec2 uv = fragTexCoord;

    float wave = sin(uv.y * params.frequency + params.time) * params.amplitude;
    uv.x += wave;

    outColor = texture(texSampler, uv);
}
