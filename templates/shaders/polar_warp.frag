#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform Params {
    float radial_scale;
    float angular_velocity;
    float chroma_split;
};

void main() {
    vec2 center = fragTexCoord - 0.5;

    float dist = length(center);
    float angle = atan(center.y, center.x);

    float r = dist + radial_scale * 0.1 * sin(dist * 20.0 + angular_velocity);
    float theta = angle + angular_velocity * 0.3 + sin(dist * 8.0) * radial_scale * 0.5;

    vec2 uv_r = vec2(0.5 + r * cos(theta - chroma_split), 0.5 + r * sin(theta - chroma_split));
    vec2 uv_g = vec2(0.5 + r * cos(theta), 0.5 + r * sin(theta));
    vec2 uv_b = vec2(0.5 + r * cos(theta + chroma_split), 0.5 + r * sin(theta + chroma_split));

    float red = texture(texSampler, uv_r).r;
    float green = texture(texSampler, uv_g).g;
    float blue = texture(texSampler, uv_b).b;

    outColor = vec4(red, green, blue, 1.0);
}
