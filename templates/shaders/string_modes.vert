#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in float inSize;

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform Params {
    float time;
    float mode_amps[8];
};

void main() {
    float t = float(gl_VertexIndex) / 512.0;

    float y = 0.0;
    float total_energy = 0.0;

    for (int i = 0; i < 8; i++) {
        float harmonic = float(i + 1);
        float standing = sin(harmonic * 3.14159 * t);
        float temporal = cos(time * harmonic * 0.8 + float(i) * 0.2);
        y += mode_amps[i] * standing * temporal;
        total_energy += mode_amps[i];
    }

    float x = t * 1.8 - 0.9;

    gl_Position = vec4(x, y * 0.5, 0.0, 1.0);
    gl_PointSize = 3.0 + total_energy * 6.0;

    float mode_hue = 0.0;
    for (int i = 0; i < 8; i++) {
        mode_hue += mode_amps[i] * float(i) / 8.0;
    }
    mode_hue = total_energy > 0.001 ? mode_hue / total_energy : 0.5;

    fragColor = vec3(
            0.3 + 0.7 * sin(mode_hue * 6.28),
            0.4 + 0.6 * sin(mode_hue * 6.28 + 2.094),
            0.8 + 0.2 * sin(mode_hue * 6.28 + 4.189)
        );
}
