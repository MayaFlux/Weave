#version 450

layout(push_constant) uniform Params {
    float aspect;
    float displacement_scale;
};

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec2 in_texcoord;
layout(location = 0) out vec2 uv;

void main() {
    vec2 dir = normalize(in_position.xy);
    vec2 displaced = in_position.xy + dir * displacement_scale;
    displaced.x /= aspect;
    gl_Position = vec4(displaced, 0.0, 1.0);
    uv = in_texcoord;
}
