#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_color;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normals;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 2) uniform DirectionalLight {
    vec3 position;
    float intensity;
    vec3 color;
} directional;

void main() {
    vec3 light_direction = normalize(directional.position.xyz + subpassLoad(u_normals).xyz);
    float directional_intensity = max(dot(normalize(subpassLoad(u_normals).xyz), light_direction), 0.0);
    vec3 directional_color = directional_intensity * directional.intensity * directional.color;

    vec3 combined = directional_color * subpassLoad(u_color).rgb;

    f_color = vec4(combined, 1.0);
}
