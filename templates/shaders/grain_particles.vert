#version 450

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform PushConstants {
    float grain_density;
    float pitch_dispersion;
    float spatial_spread;
    float time;
} params;

float hash(float n) {
    return fract(sin(n) * 43758.5453);
}

void main() {
    float id = float(gl_VertexIndex);

    float angle = hash(id * 0.1 + params.time) * 6.28318;
    float dist = hash(id * 0.2 + params.time * 0.5) * params.spatial_spread;

    float x = cos(angle) * dist;
    float y = sin(angle) * dist;

    gl_Position = vec4(x, y, 0.0, 1.0);

    float hue = hash(id * 0.3) * params.pitch_dispersion;
    fragColor = vec3(
            0.5 + hue * 0.5,
            0.7 - hue * 0.3,
            0.9 - hue * 0.4
        );

    gl_PointSize = 2.0 + params.grain_density * 6.0;
}
