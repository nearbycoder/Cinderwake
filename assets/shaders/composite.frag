#version 100
precision mediump float;
varying mediump vec2 uv;
uniform sampler2D Texture;
uniform sampler2D Bloom;
uniform vec4 Grade;
uniform vec4 Lights[8];
uniform vec4 LightColors[8];
void main() {
    vec3 source = texture2D(Texture, uv).rgb;
    vec3 glow = texture2D(Bloom, uv).rgb;
    // World scene stays nearest sampled, and the soft bloom only adds light.
    vec3 c = source * Grade.rgb;
    c = (c - 0.18) * 1.035 + 0.18;
    c += glow * Grade.a * (1.0 - c * 0.42);
    vec2 screenUv = vec2(uv.x, 1.0 - uv.y);
    vec2 world = screenUv * vec2(640.0, 360.0);
    for (int i = 0; i < 8; i++) {
        vec2 delta = world - Lights[i].xy;
        float falloff = max(0.0, 1.0 - length(delta) / max(1.0, Lights[i].z));
        c += LightColors[i].rgb * falloff * falloff * Lights[i].w * (0.14 + source * 0.55);
    }
    vec2 centered = screenUv * 2.0 - 1.0;
    float vignette = smoothstep(0.38, 1.65, dot(centered, centered));
    c *= 1.0 - vignette * 0.16;
    gl_FragColor = vec4(clamp(c, 0.0, 1.0), 1.0);
}
