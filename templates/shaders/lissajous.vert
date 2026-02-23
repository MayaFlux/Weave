#version 450

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform PushConstants {
    float time;
    float freq_x;
    float freq_y;
    float phase_offset;
} params;

void main() {
    float t = float(gl_VertexIndex) * 0.02 + params.time;

    float x = sin(params.freq_x * t) * 0.8;
    float y = cos(params.freq_y * t + params.phase_offset) * 0.8;

    gl_Position = vec4(x, y, 0.0, 1.0);

    float intensity = (sin(t * 2.0) + 1.0) * 0.5;
    fragColor = vec3(intensity, 1.0 - intensity, 0.5);

    gl_PointSize = 6.0;
}
