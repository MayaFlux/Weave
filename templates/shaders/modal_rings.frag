#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform Params {
    float mode_count;
    float time;
    float amps[16];
    float ratios[16];
};

void main() {
    vec2 uv = fragTexCoord;
    vec2 center = uv - 0.5;
    float dist = length(center);
    float angle = atan(center.y, center.x);

    float displacement = 0.0;
    float color_shift = 0.0;

    int count = int(mode_count);
    for (int i = 0; i < count && i < 16; i++) {
        float amp = amps[i];
        float ratio = ratios[i];

        float ring_radius = float(i + 1) / float(count) * 0.5;
        float ring_width = 0.02 + amp * 0.05;

        float ring_dist = abs(dist - ring_radius);
        float ring_mask = smoothstep(ring_width, 0.0, ring_dist);

        displacement += ring_mask * amp * 0.08 * sin(angle * ratio + time * (float(i) + 1.0));
        color_shift += ring_mask * amp;
    }

    uv += center * displacement;

    vec4 tex = texture(texSampler, uv);

    tex.r += color_shift * 0.4;
    tex.b += color_shift * 0.2;
    tex.g -= color_shift * 0.1;

    outColor = tex;
}
