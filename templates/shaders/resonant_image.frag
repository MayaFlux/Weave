#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform Params {
    float energy;
    float centroi;
    float spread;
    float decay_phase;
    float attack;
};

/// @brief Attempt hue rotation in RGB space via Rodrigues' rotation
vec3 rotate_hue(vec3 col, float angle) {
    float c = cos(angle);
    float s = sin(angle);
    vec3 k = vec3(0.57735);
    return col * c + cross(k, col) * s + k * dot(k, col) * (1.0 - c);
}

void main() {
    vec2 uv = fragTexCoord;
    vec2 center = uv - 0.5;

    float dist_warp = energy * 0.08 * sin(length(center) * 30.0 - decay_phase * 10.0);
    uv.x += dist_warp * spread;
    uv.y += dist_warp * (1.0 - spread);

    float scale_x = 1.0 + spread * 0.1;
    float scale_y = 1.0 - spread * 0.05;
    uv = 0.5 + (uv - 0.5) * vec2(scale_x, scale_y);

    vec4 tex = texture(texSampler, uv);
    vec3 col = tex.rgb;

    col = rotate_hue(col, centroi * 2.0);

    float edge_x = abs(dFdx(tex.r)) + abs(dFdx(tex.g)) + abs(dFdx(tex.b));
    float edge_y = abs(dFdy(tex.r)) + abs(dFdy(tex.g)) + abs(dFdy(tex.b));
    float edges = (edge_x + edge_y) * attack * 8.0;
    col += vec3(edges);

    float feedback = decay_phase * 0.3;
    vec2 feedback_uv = 0.5 + (uv - 0.5) * (1.0 + feedback * 0.02);
    vec3 fb_col = texture(texSampler, feedback_uv).rgb;
    col = mix(col, fb_col, feedback * 0.4);

    outColor = vec4(col, tex.a);
}
