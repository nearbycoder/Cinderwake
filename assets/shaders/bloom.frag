#version 100
precision mediump float;
varying mediump vec2 uv;
uniform sampler2D Texture;
uniform vec2 Direction;
uniform float Extract;
vec3 sampleLight(vec2 p) {
    vec3 c = texture2D(Texture, clamp(p, vec2(0.001), vec2(0.999))).rgb;
    float peak = max(c.r, max(c.g, c.b));
    // Soft knee avoids a threshold popping on animated flame frames.
    return c * mix(1.0, smoothstep(0.55, 0.92, peak), Extract);
}
void main() {
    vec3 glow = sampleLight(uv) * 0.227027;
    glow += (sampleLight(uv + Direction) + sampleLight(uv - Direction)) * 0.194595;
    glow += (sampleLight(uv + Direction * 2.0) + sampleLight(uv - Direction * 2.0)) * 0.121622;
    glow += (sampleLight(uv + Direction * 3.0) + sampleLight(uv - Direction * 3.0)) * 0.054054;
    glow += (sampleLight(uv + Direction * 4.0) + sampleLight(uv - Direction * 4.0)) * 0.016216;
    gl_FragColor = vec4(glow, 1.0);
}
