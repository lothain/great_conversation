// Shared by every globe shader. Mirrors `aph_globe::clock::GpuClock` and the rules in
// `aph_scene::time` (which has the tests).

struct Clock {
    year: f32,
    window: f32,
    seconds: f32,
    reveal: f32,
    px_ratio: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

// Opacity for something that appeared `age` years ago: 1 while fresh, fading over the
// last fifth of the window, 0 outside it (and before it appears).
fn fade(clock: Clock, age: f32) -> f32 {
    if age < 0.0 { return 0.0; }
    if clock.window <= 0.0 { return 1.0; }
    return 1.0 - smoothstep(clock.window * 0.8, clock.window, age);
}

// Clip-space position to logical (CSS) pixels from the viewport centre.
fn to_px(clip: vec4<f32>, viewport: vec4<f32>, px_ratio: f32) -> vec2<f32> {
    return clip.xy / clip.w * viewport.zw * 0.5 / px_ratio;
}

// Offset a clip-space position by a vector in logical pixels.
fn offset_px(clip: vec4<f32>, px: vec2<f32>, viewport: vec4<f32>, px_ratio: f32) -> vec4<f32> {
    let ndc = px * px_ratio / (viewport.zw * 0.5);
    return vec4(clip.xy + ndc * clip.w, clip.z, clip.w);
}
