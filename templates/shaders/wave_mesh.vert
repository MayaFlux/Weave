#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in float inSize;

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform Params {
    float time;
    float freq_x;
    float freq_y;
    float amp_x;
    float amp_y;
    float grid_size;
};

void main() {
    vec3 pos = inPosition;

    float dx = sin(pos.x * freq_x + time * 2.0) * amp_x;
    float dy = sin(pos.y * freq_y + time * 1.4) * amp_y;
    float cross_term = sin((pos.x + pos.y) * (freq_x + freq_y) * 0.3 + time * 3.0)
            * amp_x * amp_y * 0.3;

    float displacement = (dx + dy + cross_term) * 0.15;

    pos.y += displacement;

    gl_Position = vec4(pos.xy, 0.0, 1.0);
    gl_PointSize = inSize + abs(displacement) * 8.0;

    float wave_color = displacement * 3.0 + 0.5;
    fragColor = vec3(
            inColor.r + wave_color * 0.3,
            inColor.g,
            inColor.b - wave_color * 0.2
        );
}
