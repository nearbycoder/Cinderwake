#version 100
// Texel positions reach 2560, beyond what mediump promises.
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying mediump vec2 uv;
uniform sampler2D Texture;
uniform sampler2D Bloom;
uniform vec4 Grade;
// The renderer sets LIGHTS for each fidelity step.
#define LIGHTS 8
uniform vec4 Lights[LIGHTS];
uniform vec4 LightColors[LIGHTS];
// The scene's size in texels, and window pixels per scene texel.
uniform vec2 SceneSize;
uniform float Scale;
// Enlarging, each scene texel covers a whole number of window pixels plus a
// fraction. Nearest sampling gives the fraction to one side, so some texels
// are drawn a pixel wider than others; here each texel is flat across its
// own span and only the pixel on its seam blends with the next. At whole-
// number scales that pixel lands exactly, so it matches nearest sampling.
vec3 scene(vec2 at) {
    if (Scale < 1.0) {
        return texture2D(Texture, at).rgb;
    }
    vec2 texel = at * SceneSize;
    vec2 cell = floor(texel - 0.5);
    vec2 offset = texel - 0.5 - cell;
    // The seam between the two texels is at 0.5, one window pixel wide.
    vec2 blend = clamp((offset - 0.5) * Scale + 0.5, 0.0, 1.0);
    vec2 a = (cell + 0.5) / SceneSize;
    vec2 b = (cell + 1.5) / SceneSize;
    vec3 top = mix(texture2D(Texture, a).rgb, texture2D(Texture, vec2(b.x, a.y)).rgb, blend.x);
    vec3 bottom = mix(texture2D(Texture, vec2(a.x, b.y)).rgb, texture2D(Texture, b).rgb, blend.x);
    return mix(top, bottom, blend.y);
}
void main() {
    vec3 source = scene(uv);
    vec3 glow = texture2D(Bloom, uv).rgb;
    // The world keeps its hard pixels, and the soft bloom only adds light.
    vec3 c = source * Grade.rgb;
    c = (c - 0.18) * 1.035 + 0.18;
    c += glow * Grade.a * (1.0 - c * 0.42);
    vec2 screenUv = vec2(uv.x, 1.0 - uv.y);
    vec2 world = screenUv * vec2(640.0, 360.0);
    for (int i = 0; i < LIGHTS; i++) {
        vec2 delta = world - Lights[i].xy;
        float falloff = max(0.0, 1.0 - length(delta) / max(1.0, Lights[i].z));
        c += LightColors[i].rgb * falloff * falloff * Lights[i].w * (0.14 + source * 0.55);
    }
    vec2 centered = screenUv * 2.0 - 1.0;
    float vignette = smoothstep(0.38, 1.65, dot(centered, centered));
    c *= 1.0 - vignette * 0.16;
    gl_FragColor = vec4(clamp(c, 0.0, 1.0), 1.0);
}
