#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    float time;
    float spiral_intensity;
    float zoom;
} params;

void main() {
    vec2 uv = fragTexCoord;
    vec2 center = vec2(0.5, 0.5);

    vec2 delta = uv - center;
    float dist = length(delta);
    float angle = atan(delta.y, delta.x);

    float spiral = angle + dist * 10.0 + params.time;

    vec2 distorted;
    distorted.x = dist * cos(spiral);
    distorted.y = dist * sin(spiral);

    distorted = distorted * params.zoom * params.spiral_intensity + center;

    outColor = texture(texSampler, distorted);
}
