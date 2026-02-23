#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    float time;
    float segments;
    float rotation;
} params;

void main() {
    vec2 uv = fragTexCoord;
    vec2 center = vec2(0.5, 0.5);

    vec2 delta = uv - center;
    float angle = atan(delta.y, delta.x) + params.rotation;
    float dist = length(delta);

    float segment_angle = 6.28318 / params.segments;
    angle = mod(angle, segment_angle);

    if (mod(floor(angle / segment_angle), 2.0) > 0.5) {
        angle = segment_angle - angle;
    }

    vec2 kaleidoscope_uv;
    kaleidoscope_uv.x = 0.5 + dist * cos(angle);
    kaleidoscope_uv.y = 0.5 + dist * sin(angle);

    outColor = texture(texSampler, kaleidoscope_uv);
}
