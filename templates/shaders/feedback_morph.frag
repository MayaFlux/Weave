#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform Params {
    float cursor_x;
    float cursor_y;
    float feedback_amount;
    float time;
};

void main() {
    vec2 uv = fragTexCoord;
    vec2 cursor = vec2(cursor_x * 0.5 + 0.5, cursor_y * -0.5 + 0.5);

    vec2 to_cursor = cursor - uv;
    float dist = length(to_cursor);

    float warp_strength = feedback_amount * 0.15 / (dist + 0.1);
    uv += to_cursor * warp_strength * sin(dist * 20.0 + time * 5.0);

    float zoom = 1.0 + feedback_amount * 0.02 * sin(time * 3.0);
    uv = cursor + (uv - cursor) * zoom;

    vec4 tex = texture(texSampler, uv);

    float angular = atan(to_cursor.y, to_cursor.x);
    float ring = sin(dist * 40.0 - time * 8.0) * 0.5 + 0.5;
    float sector = sin(angular * 6.0 + time * 2.0) * 0.5 + 0.5;

    float overlay = ring * sector * feedback_amount * 0.3;

    tex.rgb += vec3(overlay * 0.4, overlay * 0.2, overlay * 0.6);

    outColor = tex;
}
