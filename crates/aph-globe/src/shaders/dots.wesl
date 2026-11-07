// Screen-space SDF discs for aphorisms (religion-coloured) and ideas (gold, glowing).
//
// Vertex: each quad's four corners share one centre; expand them by the radius in pixels.
// New dots pop in with a decaying overshoot. Fragment: signed distance to the disc edge,
// anti-aliased with fwidth, with a parchment ring, an idea glow, a religion mask that dims
// other faiths, and a pulsing highlight ring for the node the host points at.

import bevy_pbr::render::mesh_view_bindings::view;
import aph_globe::shaders::common::{Clock, fade, offset_px};

struct Style {
    religion_bits: u32,
    masked: u32,
    highlight: i32,
    _pad: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> clock: Clock;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> style: Style;

struct Vertex {
    @location(0) center: vec3<f32>,
    @location(1) corner: vec2<f32>,
    @location(2) params: vec4<f32>,   // from, until, is_idea, radius_px
    @location(3) fill: vec4<f32>,
    @location(4) ids: vec2<f32>,      // religion bit (-1 none), node index
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    // Position inside the quad in units of the disc radius.
    @location(0) p: vec2<f32>,
    @location(1) @interpolate(flat) fill: vec4<f32>,
    @location(2) @interpolate(flat) idea: f32,
    @location(3) @interpolate(flat) alpha: f32,
    @location(4) @interpolate(flat) highlighted: f32,
}

// Quads extend past the disc to leave room for the ring, the glow and the highlight.
const EXTENT: f32 = 2.6;

fn religion_weight(bit: f32) -> f32 {
    if style.masked == 0u { return 1.0; }
    if bit < 0.0 { return 0.25; }
    let on = (style.religion_bits >> u32(bit)) & 1u;
    return select(0.25, 1.0, on == 1u);
}

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let age = clock.year - v.params.x;
    let visible = age >= 0.0;
    let alpha = select(0.0, fade(clock, max(clock.year - v.params.y, 0.0)), visible);
    let highlighted = select(0.0, 1.0, i32(v.ids.y) == style.highlight);

    // Pop: overshoot to 1.6x and settle within a few years of appearing.
    let pop = 1.0 + 0.6 * exp(-max(age, 0.0) * 0.35) * select(0.0, 1.0, visible);
    let radius = v.params.w * pop * select(0.0, 1.0, alpha > 0.0) * (1.0 + 0.3 * highlighted);

    let clip = view.clip_from_world * vec4(v.center, 1.0);
    out.clip = offset_px(clip, v.corner * radius * EXTENT, view.viewport, clock.px_ratio);
    out.p = v.corner * EXTENT;
    out.fill = v.fill;
    out.idea = v.params.z;
    out.alpha = alpha * select(religion_weight(v.ids.x), 1.0, v.params.z > 0.5);
    out.highlighted = highlighted;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = length(in.p) - 1.0;           // signed distance to the disc edge, in radii
    let aa = fwidth(d);
    let disc = 1.0 - smoothstep(-aa, aa, d);
    let ring = smoothstep(-0.32 - aa, -0.32 + aa, d);   // outer band of the disc

    let parchment = vec3(0.86, 0.79, 0.62);
    var rgb = mix(in.fill.rgb, parchment, ring * 0.85);
    var a = disc;

    if in.idea > 0.5 {
        let gold = vec3(0.79, 0.59, 0.12);
        let glow = exp(-max(d, 0.0) * 3.0) * (0.55 + 0.15 * sin(clock.seconds * 2.0));
        rgb = mix(gold * 1.2, rgb, disc);
        a = max(disc, glow * (1.0 - disc));
    }

    if in.highlighted > 0.5 {
        // A thin accent ring just outside the disc, breathing slowly.
        let r = 0.45 + 0.08 * sin(clock.seconds * 4.0);
        let band = 1.0 - smoothstep(0.06 - aa, 0.06 + aa, abs(d - r));
        let accent = vec3(0.54, 0.18, 0.11);
        rgb = mix(rgb, accent, band * (1.0 - disc));
        a = max(a, band);
    }

    if a * in.alpha < 0.003 {
        discard;
    }
    return vec4(rgb, a * in.alpha);
}
