#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in float inSize;

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform Params {
    float time;
    float radius_mod;
    float energy;
    float aspect;
};

void main() {
    float angle = float(gl_VertexIndex) / 360.0 * 6.28318;

    float base_r = 0.5 + radius_mod * 0.2;
    float perturb = sin(angle * 7.0 + time * 3.0) * energy * 0.15
            + sin(angle * 13.0 - time * 1.7) * energy * 0.08
            + sin(angle * 3.0 + time * 5.0) * energy * 0.05;
    float r = base_r + perturb;

    float x = cos(angle) * r / aspect;
    float y = sin(angle) * r;

    gl_Position = vec4(x, y, 0.0, 1.0);
    gl_PointSize = 4.0 + energy * 12.0;

    float brightness = 0.5 + energy * 0.5;
    float hue = angle / 6.28318;
    fragColor = vec3(
            brightness * (0.5 + 0.5 * sin(hue * 6.28318)),
            brightness * (0.5 + 0.5 * sin(hue * 6.28318 + 2.094)),
            brightness * (0.5 + 0.5 * sin(hue * 6.28318 + 4.189))
        );
}
