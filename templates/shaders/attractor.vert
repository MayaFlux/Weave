#version 450

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform PushConstants {
    float time;
    float sigma;
    float rho;
    float beta;
} params;

void main() {
    float t = float(gl_VertexIndex) * 0.01;

    vec3 pos = vec3(0.1, 0.0, 0.0);

    float dt = 0.01;
    for (int i = 0; i < gl_VertexIndex; i++) {
        float dx = params.sigma * (pos.y - pos.x);
        float dy = pos.x * (params.rho - pos.z) - pos.y;
        float dz = pos.x * pos.y - params.beta * pos.z;

        pos.x += dx * dt;
        pos.y += dy * dt;
        pos.z += dz * dt;
    }

    float phase = params.time + t;

    vec2 screen_pos = pos.xy * 0.02;
    gl_Position = vec4(screen_pos, 0.0, 1.0);

    float alpha = float(gl_VertexIndex) / 50000.0;
    fragColor = vec3(alpha * 0.8 + 0.2);

    gl_PointSize = 2.0;
}
