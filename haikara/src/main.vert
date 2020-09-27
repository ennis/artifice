#version 450
layout(location=0) in vec3 i_position;
layout(location=1) in vec3 i_normal;

layout(location=0) out vec3 f_normal;

// World->Light
uniform mat4 lightMatrix;
// Model->World
uniform mat4 modelMatrix;
// World->Clip
uniform mat4 viewProjMatrix;

void main() {
    f_normal = (lightMatrix*modelMatrix*vec4(i_normal,0.0)).xyz;
    gl_Position = viewProjMatrix*modelMatrix*vec4(i_position,1.0);
}