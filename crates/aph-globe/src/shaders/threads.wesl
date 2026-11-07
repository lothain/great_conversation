// "Responds to" threads as screen-space ribbons.
//
// Vertex: project the centerline point and its neighbours, take the screen-space tangent,
// and push each edge vertex out along the screen normal. Widths are in logical pixels
// (scaled by the device pixel ratio), so lines look the same on any display and zoom.
// Near the target end the ribbon widens into an arrowhead and tapers to a point; its
// length is measured in pixels to the arc's end, so short and long arcs get the same arrow.
//
// Fragment: anti-aliased core with a parchment outline for legibility; faint at the
// responder, dark at the work being answered; drawn in over `reveal` years; slow pulses
// of ink flowing toward the target; weak evidence (alludes, influenced) dashed.

import bevy_pbr::render::mesh_view_bindings::view;
import aph_globe::shaders::common::{Clock, fade, offset_px, to_px};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> clock: Clock;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) prev: vec3<f32>,
    @location(2) next: vec3<f32>,
    @location(3) line: vec4<f32>,   // t, side, year, width_px
    @location(4) ink: vec4<f32>,    // linear rgb, evidence kind / 4
    @location(5) end: vec3<f32>,    // the arc's last point
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) t: f32,
    // Signed distance from the centerline in logical px, and this vertex's half-width.
    @location(1) side_px: f32,
    @location(2) half_width: f32,
    // Straight-line distance to the arc's end in logical px.
    @location(3) to_end_px: f32,
    @location(4) @interpolate(flat) age: f32,
    @location(5) @interpolate(flat) ink: vec4<f32>,
}

const ARROW_PX: f32 = 13.0;     // arrowhead length
const ARROW_WIDTH: f32 = 3.2;   // arrowhead base, as a multiple of the line width
const OUTLINE_PX: f32 = 1.4;    // parchment outline around the ink

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    let r = clock.px_ratio;
    let clip = view.clip_from_world * vec4(v.position, 1.0);
    let here = to_px(clip, view.viewport, r);
    let a = to_px(view.clip_from_world * vec4(v.prev, 1.0), view.viewport, r);
    let b = to_px(view.clip_from_world * vec4(v.next, 1.0), view.viewport, r);
    let end = to_px(view.clip_from_world * vec4(v.end, 1.0), view.viewport, r);
    let tangent = b - a;
    let dir = select(vec2(1.0, 0.0), normalize(tangent), dot(tangent, tangent) > 1e-8);
    let normal = vec2(-dir.y, dir.x);

    // Arrowhead: full base width ARROW_PX from the end, tapering to nothing at the end.
    let to_end = length(end - here);
    let in_arrow = to_end < ARROW_PX && v.line.x > 0.5;
    let width = select(v.line.w, v.line.w * ARROW_WIDTH * (to_end / ARROW_PX), in_arrow);
    let half_width = width * 0.5;

    // Room for the outline and one pixel of anti-aliasing on each side.
    let reach = half_width + OUTLINE_PX + 1.0;

    var out: VertexOutput;
    out.clip = offset_px(clip, normal * v.line.y * reach, view.viewport, r);
    out.t = v.line.x;
    out.side_px = v.line.y * reach;
    out.half_width = half_width;
    out.to_end_px = to_end;
    out.age = clock.year - v.line.z;
    out.ink = v.ink;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.age < 0.0 {
        discard;   // not yet said
    }
    let drawn = select(1.0, in.age / clock.reveal, clock.reveal > 0.0);
    if in.t > drawn {
        discard;   // still drawing in from the responder
    }
    let faded = fade(clock, in.age);

    // Weak evidence is dashed: 7px on, 5px off, measured back from the arrow.
    let kind = in.ink.a * 4.0;
    let dashed = kind > 2.5 && in.to_end_px > ARROW_PX;
    if dashed && fract(in.to_end_px / 12.0) > 0.58 {
        discard;
    }

    // Coverage of the ink and of the ink-plus-outline, anti-aliased across one pixel.
    let d = abs(in.side_px);
    let ink_cov = 1.0 - smoothstep(in.half_width - 0.5, in.half_width + 0.5, d);
    let outline_cov = 1.0 - smoothstep(in.half_width + OUTLINE_PX - 0.5, in.half_width + OUTLINE_PX + 0.5, d);

    // Faint at the responder, dark at the one being answered.
    let strength = mix(0.35, 1.0, in.t);

    // Pulses of ink flowing toward the target, brightest while the thread is new.
    let flow = fract(in.t * 6.0 - clock.seconds * 0.35);
    let pulse = smoothstep(0.0, 0.15, flow) * (1.0 - smoothstep(0.15, 0.45, flow));
    let freshness = exp(-in.age / 60.0);
    var ink = in.ink.rgb * (1.0 + pulse * (0.3 + freshness));

    // A bright tip while drawing in.
    let tip = (1.0 - smoothstep(0.0, 0.025, drawn - in.t)) * step(drawn, 0.999);
    ink = mix(ink, vec3(1.0, 0.85, 0.5), tip);

    let paper = vec3(0.93, 0.87, 0.72);
    let rgb = mix(paper, ink, ink_cov);
    let alpha = max(ink_cov * strength, outline_cov * 0.55 * strength) * faded;
    if alpha < 0.003 {
        discard;
    }
    return vec4(rgb, alpha);
}
