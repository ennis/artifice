#version 450

in vec2 pos;
out vec4 o_color;

layout(binding=0) uniform sampler2D tex;

void main() {
    vec2 texcoord = 0.5 * pos - vec2(0.5);
    o_color = texture(tex, texcoord);
}