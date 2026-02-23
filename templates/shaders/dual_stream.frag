#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform Params {
    float aspect;
    float displacement_a;
    float displacement_b;
};

void main() {
    vec2 center = fragTexCoord - 0.5;
    center.x *= aspect;

    float dist = length(center);
    float angle = atan(center.y, center.x);

    float r = dist + displacement_a * 0.15 * sin(angle * 3.0);
    float theta = angle + displacement_b * 0.4;

    vec2 warped = vec2(
            0.5 + r * cos(theta) / aspect,
            0.5 + r * sin(theta)
        );

    outColor = texture(texSampler, warped);
}
