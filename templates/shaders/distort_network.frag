#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    float density;
    float coherence;
    float spectral_tilt;
} params;

void main() {
    vec2 uv = fragTexCoord;

    float wave1 = sin(uv.x * 20.0 * params.coherence) * params.density * 0.1;
    float wave2 = cos(uv.y * 15.0 * params.coherence) * params.density * 0.1;

    vec2 distortion = vec2(wave1, wave2) * (1.0 + params.spectral_tilt);

    vec3 color;
    float aberration = (1.0 - params.coherence) * 0.02;
    color.r = texture(texSampler, uv + distortion + vec2(aberration, 0.0)).r;
    color.g = texture(texSampler, uv + distortion).g;
    color.b = texture(texSampler, uv + distortion - vec2(aberration, 0.0)).b;

    outColor = vec4(color, 1.0);
}
