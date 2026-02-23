#version 450

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform PushConstants {
    float time;
    float radius_scale;
    float turn_speed;
} params;

void main() {
    float t = float(gl_VertexIndex) * 0.1;

    float radius = t * 0.05 * params.radius_scale;
    float angle = t + params.time * params.turn_speed;

    float x = radius * cos(angle);
    float y = radius * sin(angle);

    gl_Position = vec4(x, y, 0.0, 1.0);

    float hue = float(gl_VertexIndex) / 500.0;
    fragColor = vec3(
            sin(hue * 6.28) * 0.5 + 0.5,
            cos(hue * 6.28) * 0.5 + 0.5,
            sin(hue * 9.42) * 0.5 + 0.5
        );

    gl_PointSize = 8.0;
}
