#version 450

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform PushConstants {
    float chaos_x;
    float chaos_y;
    float orbit_scale;
    float distortion;
} params;

void main() {
    float t = float(gl_VertexIndex) * 0.1;

    float radius = t * 0.05;
    float angle = t;

    radius *= (1.0 + params.chaos_x * params.distortion);
    angle += params.chaos_y * params.orbit_scale;

    float x = radius * cos(angle);
    float y = radius * sin(angle);

    gl_Position = vec4(x, y, 0.0, 1.0);

    float hue = (params.chaos_x + params.chaos_y) * 0.5;
    fragColor = vec3(
            abs(params.chaos_x),
            abs(params.chaos_y),
            abs(hue)
        );

    gl_PointSize = 6.0 + abs(params.chaos_x) * 4.0;
}
