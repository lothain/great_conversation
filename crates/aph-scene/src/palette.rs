//! Colours shared by the renderer and the HTML widgets (exported to JS as hex).

use aph_data::{graph::Valence, vocab::ReligionFamily};

/// An sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    #[must_use]
    pub const fn hex(v: u32) -> Self {
        Self((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    #[must_use]
    pub fn css(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    /// Linear-light components, for vertex colours.
    #[must_use]
    pub fn linear(self) -> [f32; 3] {
        let f = |c: u8| {
            let c = f32::from(c) / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        [f(self.0), f(self.1), f(self.2)]
    }
}

#[must_use]
pub fn religion(family: ReligionFamily) -> Rgb {
    use ReligionFamily as R;
    Rgb::hex(match family {
        R::Christianity => 0x7b4a2e,
        R::Judaism => 0x3d5a80,
        R::Islam => 0x4f6b3a,
        R::Hinduism => 0xc07a2c,
        R::Buddhism => 0xb08a3a,
        R::GreekPolytheism => 0x8c6d9e,
        R::RomanPolytheism => 0x9e5f6d,
        R::Zoroastrianism => 0xa0522d,
        R::Manichaeism => 0x7a5a3a,
        R::Gnosticism => 0x6b4f7a,
        R::Deism => 0x5e7f7f,
        R::Pantheism => 0x6f8f4f,
        R::Agnosticism => 0x8a8a7a,
        R::Atheism | R::Secular => 0x555555,
        _ => 0x9c8f78,
    })
}

/// Colour for an aphorism whose author's religion is unknown.
pub const INK: Rgb = Rgb::hex(0x5a4632);
/// @mentions: echoes between utterances.
pub const MENTION: Rgb = Rgb::hex(0x2f4c72);
/// Gold, for new ideas.
pub const GOLD: Rgb = Rgb::hex(0xc9971f);

#[must_use]
pub fn valence(v: Valence) -> Rgb {
    match v {
        Valence::Adopts => Rgb::hex(0x4d5a2a),
        Valence::Mixed => Rgb::hex(0x9a6a1a),
        Valence::Rejects => Rgb::hex(0x8a2f1d),
    }
}

/// Region colours: muted, distinct, and plenty, so neighbouring regions never need to share.
pub const REGIONS: [Rgb; 12] = [
    Rgb::hex(0xb5603a), // terracotta
    Rgb::hex(0x6f8a3c), // olive
    Rgb::hex(0x7a5596), // plum
    Rgb::hex(0xc49a3a), // ochre
    Rgb::hex(0x4f6fa0), // slate blue
    Rgb::hex(0x3f8a7f), // teal
    Rgb::hex(0xa8506b), // rose
    Rgb::hex(0x8c5a3c), // umber
    Rgb::hex(0x5f8fa8), // steel
    Rgb::hex(0xa39a2a), // mustard
    Rgb::hex(0x8a73b0), // lavender
    Rgb::hex(0x5a8a52), // moss
];

/// A stable colour per civilization id (FNV-1a hash), so it never changes between runs.
#[must_use]
pub fn civ(id: &str) -> Rgb {
    REGIONS[civ_index(id)]
}

/// A civilization's preferred colour index in [`REGIONS`], stable across runs (FNV-1a).
#[must_use]
pub fn civ_index(id: &str) -> usize {
    let h = id.bytes().fold(0x811c_9dc5_u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
    h as usize % REGIONS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_round_trip() {
        assert_eq!(Rgb::hex(0x8a2f1d).css(), "#8a2f1d");
    }

    #[test]
    fn civ_colours_are_stable() {
        assert_eq!(civ("roman"), civ("roman"));
    }

    #[test]
    fn linear_is_darker_than_srgb() {
        let [r, ..] = Rgb::hex(0x808080).linear();
        assert!(r > 0.2 && r < 0.25);
    }
}
