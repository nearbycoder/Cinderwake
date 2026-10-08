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
// Ultra adds a wider halo, sharpening, and a soft highlight shoulder.
#define ULTRA 0
#if ULTRA
uniform sampler2D BloomWide;
#endif
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
#if ULTRA
    // Filtering the supersampled scene down softens it a little; a gentle
    // unsharp mask, one window pixel wide, brings the edges back.
    vec2 nudge = vec2(max(1.0, 1.0 / Scale)) / SceneSize;
    vec3 around = scene(uv + vec2(nudge.x, 0.0)) + scene(uv - vec2(nudge.x, 0.0))
        + scene(uv + vec2(0.0, nudge.y)) + scene(uv - vec2(0.0, nudge.y));
    source = max(source + (source - around * 0.25) * 0.3, 0.0);
#endif
    vec3 glow = texture2D(Bloom, uv).rgb;
    // The world keeps its hard pixels, and the soft bloom only adds light.
    vec3 c = source * Grade.rgb;
    c = (c - 0.18) * 1.035 + 0.18;
    c += glow * Grade.a * (1.0 - c * 0.42);
#if ULTRA
    c += texture2D(BloomWide, uv).rgb * Grade.a * 0.45 * (1.0 - c * 0.42);
#endif
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
#if ULTRA
    // Bright light rolls off over the top fifth instead of clipping flat.
    vec3 over = max(c - 0.8, 0.0);
    c = min(c, 0.8) + 0.2 * (1.0 - exp(-over / 0.2));
#endif
    gl_FragColor = vec4(clamp(c, 0.0, 1.0), 1.0);
}
