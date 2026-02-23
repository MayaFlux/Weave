#version 450

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform PushConstants {
    float time;
    float spike_frequency;
    float spike_amplitude;
    float rotation_speed;
} params;

void main() {
    int total_angles = 360;
    int total_layers = 20;

    int layer = gl_VertexIndex / total_angles;
    int angle_index = gl_VertexIndex % total_angles;

    if (layer >= total_layers) {
        gl_Position = vec4(0.0, 0.0, -1.0, 1.0);
        return;
    }

    float angle = float(angle_index) * 3.14159 / 180.0;
    float layer_phase = float(layer) * 0.1;

    float base_radius = 0.3 + float(layer) * 0.02;

    float spike_amp = params.spike_amplitude * sin(float(layer) * 0.3 + params.time);
    float spike = spike_amp * pow(abs(sin(angle * params.spike_frequency)), 3.0);

    float r = base_radius + spike;

    float rotated_angle = angle + params.time * params.rotation_speed + layer_phase;

    float x = r * cos(rotated_angle);
    float y = r * sin(rotated_angle);

    gl_Position = vec4(x, y, 0.0, 1.0);

    float brightness = 0.3 + r * 0.5;
    fragColor = vec3(brightness, brightness * 0.9, brightness * 0.1);

    if (abs(sin(angle * params.spike_frequency)) > 0.9) {
        fragColor = vec3(1.0, 0.95, 0.8);
    }

    gl_PointSize = 4.0;
}
