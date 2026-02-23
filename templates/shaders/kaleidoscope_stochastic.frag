#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    float segments;
    float rotation;
    float mirror_probability;
    float color_shift;
} params;

void main() {
    vec2 uv = fragTexCoord;
    vec2 center = vec2(0.5, 0.5);

    vec2 delta = uv - center;
    float angle = atan(delta.y, delta.x) + params.rotation;
    float dist = length(delta);

    float segment_angle = 6.28318 / params.segments;
    float segment_id = floor(angle / segment_angle);
    angle = mod(angle, segment_angle);

    if (mod(segment_id, 2.0) < params.mirror_probability) {
        angle = segment_angle - angle;
    }

    vec2 kaleidoscope_uv;
    kaleidoscope_uv.x = 0.5 + dist * cos(angle);
    kaleidoscope_uv.y = 0.5 + dist * sin(angle);

    vec4 sampled = texture(texSampler, kaleidoscope_uv);

    sampled.rgb = mix(sampled.rgb, sampled.gbr, params.color_shift);

    outColor = sampled;
}
