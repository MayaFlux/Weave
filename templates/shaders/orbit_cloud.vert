#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in float inSize;

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform Params {
    float time;
    float contraction;
    float tilt;
    float jitter;
    float aspect;
};

/// @brief Hash function for deterministic per-vertex randomness
float hash(float n) {
    return fract(sin(n) * 43758.5453);
}

void main() {
    float idx = float(gl_VertexIndex);
    float n = idx / 1000.0;

    float semi_major = 0.15 + hash(idx * 1.17) * 0.6;
    float eccentricity = hash(idx * 2.31) * 0.5;
    float inclination = (hash(idx * 3.71) - 0.5) * tilt * 2.0;
    float phase_offset = hash(idx * 4.93) * 6.28318;

    float orbital_speed = 0.5 / (semi_major * semi_major + 0.1);
    float theta = time * orbital_speed + phase_offset + jitter * sin(time * 1.3 + idx);

    float r = semi_major * (1.0 - eccentricity * eccentricity) / (1.0 + eccentricity * cos(theta));

    r *= (1.0 - contraction * 0.3);

    float x = r * cos(theta);
    float y = r * sin(theta) * cos(inclination) + r * 0.1 * sin(inclination);

    gl_Position = vec4(x / aspect, y, 0.0, 1.0);
    gl_PointSize = inSize * (1.0 + contraction * 0.5);

    float speed_color = orbital_speed * 0.3;
    fragColor = inColor * (0.6 + contraction * 0.4) + vec3(speed_color * 0.2, 0.0, speed_color * 0.1);
}
