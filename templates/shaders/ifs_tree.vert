#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in float inSize;

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform Params {
    float time;
    float branch_angle;
    float branch_scale;
    float trunk_bend;
    float energy;
};

void main() {
    int idx = gl_VertexIndex;

    vec2 pos = vec2(0.0, -0.7);
    float angle = 1.5708 + trunk_bend;
    float scale = 0.35;

    for (int depth = 0; depth < 5; depth++) {
        int bits = (idx >> (depth * 2)) & 3;

        float branch_offset = (float(bits) - 1.5) * branch_angle;
        branch_offset += sin(time * (float(depth) + 1.0) * 0.5) * energy * 0.15;

        angle += branch_offset;
        scale *= branch_scale;

        float step_len = scale * (0.8 + energy * 0.2 * sin(float(depth) * 2.0 + time));

        pos.x += cos(angle) * step_len;
        pos.y += sin(angle) * step_len;
    }

    gl_Position = vec4(pos, 0.0, 1.0);

    float depth_ratio = float(idx % 32) / 32.0;
    gl_PointSize = max(inSize * (1.0 - depth_ratio * 0.5) + energy * 3.0, 1.0);

    float brightness = 0.4 + energy * 0.6;
    fragColor = inColor * brightness;
}
