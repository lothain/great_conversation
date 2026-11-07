// Parchment globe.
//
// - Paper: 3D value-noise fbm on the sphere position (no seam, no pole pinch).
// - Graticule: every 15°, anti-aliased analytically with fwidth.
// - Coastlines: a signed distance field (negative on land). Distance in degrees divided by
//   its own screen-space derivative is distance in pixels: crisp, constant-width lines.
// - Regions: a label per texel that tiles all land (sea 0, unknown 1, civilizations 2..),
//   coloured from a palette. Where the label one pixel away differs, a region border is
//   drawn, so the pieces fit together with clean one-pixel seams at any zoom.

import bevy_pbr::render::{forward_io::VertexOutput, mesh_view_bindings::view};
import aph_globe::shaders::common::Clock;

struct Palette {
    colors: array<vec4<f32>, 64>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> clock: Clock;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var labels: texture_2d<u32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> palette: Palette;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var coasts: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var coasts_sampler: sampler;

const PAPER: vec3<f32> = vec3(0.82, 0.72, 0.50);
const INK: vec3<f32> = vec3(0.16, 0.10, 0.05);
// Must match `aph_scene::overlay::BORDER_RANGE_DEG`.
const FIELD_RANGE_DEG: f32 = 1.5;

fn hash3(p: vec3<f32>) -> f32 {
    let q = fract(p * 0.3183099 + vec3(0.1, 0.2, 0.3)) * 17.0;
    return fract(q.x * q.y * q.z * (q.x + q.y + q.z));
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let x00 = mix(hash3(i), hash3(i + vec3(1.0, 0.0, 0.0)), u.x);
    let x10 = mix(hash3(i + vec3(0.0, 1.0, 0.0)), hash3(i + vec3(1.0, 1.0, 0.0)), u.x);
    let x01 = mix(hash3(i + vec3(0.0, 0.0, 1.0)), hash3(i + vec3(1.0, 0.0, 1.0)), u.x);
    let x11 = mix(hash3(i + vec3(0.0, 1.0, 1.0)), hash3(i + vec3(1.0, 1.0, 1.0)), u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var q = p;
    for (var i = 0; i < 5; i++) {
        sum += amp * noise3(q);
        q = q * 2.03 + vec3(1.7, 9.2, 3.1);
        amp *= 0.5;
    }
    return sum;
}

// 1 on a grid line, 0 away from it; `w` is the coordinate's screen-space derivative.
fn grid(coord: f32, spacing: f32, w: f32) -> f32 {
    let d = abs(fract(coord / spacing + 0.5) - 0.5) * spacing;
    return 1.0 - smoothstep(0.0, w * 1.25, d);
}

// Region label at a longitude/latitude (degrees), wrapping in longitude.
fn label_at(lon: f32, lat: f32) -> u32 {
    let size = vec2<i32>(textureDimensions(labels));
    let u = fract((lon + 180.0) / 360.0);
    let v = clamp((90.0 - lat) / 180.0, 0.0, 0.99999);
    let texel = vec2<i32>(i32(u * f32(size.x)) % size.x, i32(v * f32(size.y)));
    return textureLoad(labels, texel, 0).r;
}

fn region_colour(label: u32) -> vec4<f32> {
    return palette.colors[min(label, 63u)];
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_position.xyz);
    let lat = degrees(asin(clamp(n.y, -1.0, 1.0)));
    let lon = degrees(atan2(n.x, n.z));

    // Screen-space size of a pixel in degrees. atan2 jumps at ±180°, so take the derivative of
    // whichever form of longitude is continuous here.
    let lon_wrapped = select(lon, lon + 360.0, lon < 0.0);
    let w_lon = min(fwidth(lon), fwidth(lon_wrapped));
    let w_lat = fwidth(lat);

    // Paper: large stains plus fine fibre.
    let stain = fbm(n * 3.0);
    let fibre = fbm(n * 60.0);
    var color = PAPER * (0.82 + 0.28 * stain) * (0.94 + 0.08 * fibre);

    // Land, from the coastline field: a warmer wash and a soft inner stroke along the coast.
    let uv = vec2((lon + 180.0) / 360.0, (90.0 - lat) / 180.0);
    let coast_deg = (textureSampleLevel(coasts, coasts_sampler, uv, 0.0).r * 255.0 - 128.0) / 127.0 * FIELD_RANGE_DEG;
    let coast_px = coast_deg / (max(fwidth(coast_deg), 1e-6) / clock.px_ratio);
    let land = 1.0 - smoothstep(-0.6, 0.6, coast_px);
    color = mix(color, color * vec3(0.86, 0.80, 0.68), land * 0.75);

    // Regions: colour the land by its label.
    let here = label_at(lon, lat);
    let region = region_colour(here);
    color = mix(color, region.rgb * (0.9 + 0.2 * stain), region.a * land);

    // Region seams: any neighbour one pixel away with a different land label.
    let dx = w_lon * clock.px_ratio * 0.75;
    let dy = w_lat * clock.px_ratio * 0.75;
    var seam = 0.0;
    for (var i = 0; i < 4; i++) {
        let o = array(vec2(dx, 0.0), vec2(-dx, 0.0), vec2(0.0, dy), vec2(0.0, -dy))[i];
        let other = label_at(lon + o.x, lat + o.y);
        seam = max(seam, select(0.0, 1.0, other != here && other != 0u && here != 0u));
    }
    let seam_ink = mix(region.rgb * 0.45, INK, 0.5);
    color = mix(color, seam_ink, seam * 0.75);

    // Inner coastal stroke and the coastline itself, over the regions.
    let inner = land * (1.0 - smoothstep(0.0, 9.0, -coast_px)) * step(-FIELD_RANGE_DEG * 0.98, coast_deg);
    color = mix(color, INK, inner * 0.12);
    let coastline = (1.0 - smoothstep(0.5, 1.3, abs(coast_px))) * step(abs(coast_deg), FIELD_RANGE_DEG * 0.98);
    color = mix(color, INK, coastline * 0.6);

    // Graticule.
    let meridians = grid(lon, 15.0, w_lon) * smoothstep(80.0, 60.0, abs(lat));
    let parallels = grid(lat, 15.0, w_lat);
    let equator = grid(lat, 180.0, w_lat * 1.6);
    let lines = max(max(meridians, parallels) * 0.22, equator * 0.4);
    color = mix(color, INK, lines);

    // Soft light from the viewer; a darker, warmer limb.
    let v = normalize(view.world_position - in.world_position.xyz);
    let facing = max(dot(n, v), 0.0);
    let limb = pow(1.0 - facing, 3.0);
    color = color * (0.72 + 0.28 * facing) * (1.0 - 0.45 * limb);

    return vec4(color, 1.0);
}
