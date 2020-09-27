#version 450

uniform int showShading;
layout(binding=0) uniform sampler3D interpolatedShading;

in vec3 f_normal;
layout(location=0) out vec4 o_diffuse;
layout(location=1) out vec4 o_normal;

void main() {
    if (showShading == 1) {
        o_diffuse = vec4(texture(interpolatedShading, 0.5*f_normal.xyz+0.5).rrr, 1.0);
    } else {
        o_diffuse = vec4(f_normal.zzz, 1.0);// dot(N,L), where, in light space, L = (0,0,1)
    }
    o_normal = vec4(normalize(f_normal),1.0);
}