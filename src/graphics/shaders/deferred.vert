#version 450
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 color;

layout(location = 0) out vec3 out_color;
layout(location = 1) out vec3 out_normal;

layout(set = 0, binding = 0) uniform VP {
    mat4 view;
    mat4 projection;
} vp;
layout(set = 1, binding = 0) uniform Model {
    mat4 model;
    mat4 normals;
} model;

void main() {
    mat4 MV = vp.view * model.model;
    gl_Position = vp.projection * MV * vec4(position, 1.0);

    out_color = color;
    out_normal = mat3(model.normals) * normal;
}