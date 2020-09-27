#version 450

// positions are going to be in clipspace already, do [-1,-1]->[1,1]
layout(location=0) in vec2 pos;
out vec2 texcoord;

uniform mat3 transform;

void main() {
    gl_Position = vec4((transform*vec3(pos,1.0)).xy, 0, 1);
}