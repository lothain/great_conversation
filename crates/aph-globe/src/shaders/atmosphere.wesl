// A warm glow around the globe's limb, drawn on a slightly larger shell.
//
// For each fragment of the shell, find how close the view ray passes to the globe's
// centre. Rays that miss the globe glow most where they graze it and fade to nothing at
// the shell's edge, so the shell itself never shows a hard outline. Rays that hit the
// globe get only a faint rim tint.

import bevy_pbr::render::{forward_io::VertexOutput, mesh_view_bindings::view};

const SHELL: f32 = 1.06;
const GLOW: vec3<f32> = vec3(0.85, 0.74, 0.52);

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let eye = view.world_position;
    let dir = normalize(in.world_position.xyz - eye);
    // Closest distance from the globe's centre to the view ray.
    let closest = length(eye - dot(eye, dir) * dir);
    var alpha: f32;
    if closest >= 1.0 {
        let t = 1.0 - (closest - 1.0) / (SHELL - 1.0);
        alpha = 0.32 * pow(clamp(t, 0.0, 1.0), 2.6);
    } else {
        alpha = 0.10 * pow(closest, 14.0);
    }
    return vec4(GLOW, alpha);
}
