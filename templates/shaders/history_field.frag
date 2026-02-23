#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform Params {
    float intensity;
    float taps[32];
};

void main() {
    vec2 uv = fragTexCoord;

    int idx = int(uv.y * 32.0);
    idx = clamp(idx, 0, 31);
    float displacement = taps[idx];

    int idx_prev = clamp(idx - 1, 0, 31);
    int idx_next = clamp(idx + 1, 0, 31);
    float slope = (taps[idx_next] - taps[idx_prev]) * 0.5;

    uv.x += displacement * intensity;
    uv.y += slope * intensity * 0.3;

    vec4 tex = texture(texSampler, uv);

    float activity = abs(displacement) * 2.0;
    tex.rgb = mix(tex.rgb, tex.rgb * vec3(1.0 + activity, 1.0, 1.0 - activity * 0.5), intensity);

    outColor = tex;
}
