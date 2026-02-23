#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    float time;
    float intensity;
    float frequency;
} params;

void main() {
    vec2 uv = fragTexCoord;
    vec2 center = vec2(0.5, 0.5);

    vec2 delta = uv - center;
    float dist = length(delta);

    float angle = atan(delta.y, delta.x);
    float spiral = angle + dist * params.frequency + params.time;

    float radial_wave = sin(dist * 20.0 - params.time * 3.0) * params.intensity * 0.1;

    float kaleidoscope = sin(angle * 6.0 + params.time) * params.intensity * 0.05;

    vec2 distorted_uv = uv;
    distorted_uv.x += sin(spiral) * params.intensity * 0.1 + radial_wave + kaleidoscope;
    distorted_uv.y += cos(spiral) * params.intensity * 0.1 + radial_wave - kaleidoscope;

    float aberration = params.intensity * 0.02;
    vec3 color;
    color.r = texture(texSampler, distorted_uv + vec2(aberration, 0.0)).r;
    color.g = texture(texSampler, distorted_uv).g;
    color.b = texture(texSampler, distorted_uv - vec2(aberration, 0.0)).b;

    outColor = vec4(color, 1.0);
}
