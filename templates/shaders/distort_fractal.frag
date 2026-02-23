#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    float time;
    float zoom;
    float rotation;
} params;

vec2 complexMultiply(vec2 a, vec2 b) {
    return vec2(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

void main() {
    vec2 uv = fragTexCoord;
    vec2 center = vec2(0.5, 0.5);

    vec2 centered = (uv - center) * params.zoom;
    float cos_r = cos(params.rotation);
    float sin_r = sin(params.rotation);
    vec2 rotated = vec2(
            centered.x * cos_r - centered.y * sin_r,
            centered.x * sin_r + centered.y * cos_r
        ) + center;

    vec2 feedback_uv = rotated;
    for (int i = 0; i < 3; i++) {
        vec2 sample_offset = texture(texSampler, feedback_uv).rg - 0.5;
        feedback_uv += sample_offset * 0.1 * params.zoom;

        feedback_uv = center + complexMultiply(feedback_uv - center,
                    vec2(cos(params.time * 0.5), sin(params.time * 0.5)));
    }

    vec3 color = texture(texSampler, feedback_uv).rgb;

    float hue_shift = length(feedback_uv - center) + params.time * 0.1;
    mat3 hue_rotate = mat3(
            cos(hue_shift), -sin(hue_shift), 0.0,
            sin(hue_shift), cos(hue_shift), 0.0,
            0.0, 0.0, 1.0
        );
    color = hue_rotate * color;

    outColor = vec4(color, 1.0);
}
