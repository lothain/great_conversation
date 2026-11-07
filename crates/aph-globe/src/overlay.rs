//! Keeps the globe's land layers in step with the shown year:
//!
//! - coastlines: a static signed distance field (baked, or computed once at startup);
//! - regions: a label per texel that tiles all land (see `aph_scene::regions`), rebuilt when
//!   the set of civilizations in effect changes, plus the palette the shader colours it with.

use aph_scene::{
    overlay,
    palette::{self, Rgb},
    regions::{self, FIRST_CIV, UNKNOWN},
};
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, ShaderType, TextureDimension, TextureFormat},
};

use crate::{clock::ShownTime, scene::Scene};

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1024;
/// Palette entries the shader holds; labels beyond it draw as unknown.
pub const PALETTE_SIZE: usize = 64;

/// Region colours by label, linear RGB with the fill strength in alpha.
#[derive(ShaderType, Debug, Clone, Copy)]
pub struct RegionPalette {
    pub colors: [Vec4; PALETTE_SIZE],
}

impl Default for RegionPalette {
    fn default() -> Self {
        Self { colors: [Vec4::ZERO; PALETTE_SIZE] }
    }
}

#[derive(Resource, Debug)]
pub struct CivOverlay {
    /// `R8Uint` region labels: 0 sea, 1 unknown, 2.. civilizations.
    pub labels: Handle<Image>,
    /// Linear R8: signed distance to the coast, negative on land. Static.
    pub coasts: Handle<Image>,
    pub palette: RegionPalette,
    land: Option<Vec<bool>>,
    shown: Option<(Vec<usize>, Option<std::collections::BTreeSet<String>>)>,
}

/// Which civilizations are at full strength (`None` = all), set by the host. Others stay on
/// the map (the land is always tiled) but are drawn faintly.
#[derive(Resource, Debug, Default)]
pub struct CivFilter(pub Option<std::collections::BTreeSet<String>>);

pub struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        let labels = images.add(blank(TextureFormat::R8Uint, &[UNKNOWN]));
        let coasts = images.add(blank(TextureFormat::R8Unorm, &[255]));
        app.insert_resource(CivOverlay { labels, coasts, palette: RegionPalette::default(), land: None, shown: None })
            .init_resource::<CivFilter>()
            .add_systems(Startup, paint_coasts)
            .add_systems(Update, repaint);
    }
}

fn blank(format: TextureFormat, pixel: &[u8]) -> Image {
    let mut image = Image::new_fill(
        Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixel,
        format,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

fn paint_coasts(scene: Res<Scene>, mut civ: ResMut<CivOverlay>, mut images: ResMut<Assets<Image>>) {
    let field = scene.baked_coasts.as_deref().and_then(decode_baked).or_else(|| {
        (!scene.atlas.land.is_empty()).then(|| overlay::coasts(&scene.atlas.land, WIDTH, HEIGHT))
    });
    let Some(field) = field else { return };
    civ.land = Some(regions::land_mask(&field));
    if let Some(mut image) = images.get_mut(&civ.coasts) {
        image.data = Some(field);
    }
}

/// The baked coastline field, if it decodes to an 8-bit image of the overlay's size.
fn decode_baked(png: &[u8]) -> Option<Vec<u8>> {
    use bevy::image::{CompressedImageFormats, ImageType};
    let image = Image::from_buffer(
        png,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        false,
        ImageSampler::Default,
        RenderAssetUsages::MAIN_WORLD,
    )
    .inspect_err(|e| warn!("baked coasts unreadable, computing instead: {e}"))
    .ok()?;
    if image.width() != WIDTH || image.height() != HEIGHT {
        warn!("baked coasts are {}x{}, not {WIDTH}x{HEIGHT}; computing instead", image.width(), image.height());
        return None;
    }
    // Grayscale PNGs may arrive as one channel or expanded to RGBA; keep the first channel.
    let bytes_per_texel = image.texture_descriptor.format.block_copy_size(None)? as usize;
    let data = image.data?;
    Some(data.chunks_exact(bytes_per_texel).map(|texel| texel[0]).collect())
}

/// Linear RGB plus fill strength.
fn swatch(rgb: Rgb, strength: f32) -> Vec4 {
    let [r, g, b] = rgb.linear();
    Vec4::new(r, g, b, strength)
}

/// Fill strength of a region at full emphasis, and when the host has masked it out.
const FILL: f32 = 0.5;
const MASKED: f32 = 0.10;
/// Unknown land: a neutral, muted wash.
const UNKNOWN_COLOUR: Rgb = Rgb::hex(0x8f8676);

fn repaint(scene: Res<Scene>, shown: Res<ShownTime>, filter: Res<CivFilter>, mut civ: ResMut<CivOverlay>, mut images: ResMut<Assets<Image>>) {
    let Some(land) = civ.land.as_ref() else { return };
    let civs = &scene.atlas.civs;
    let active = overlay::active(civs, shown.0.year.floor() as i32);
    let key = (active, filter.0.clone());
    if civ.shown.as_ref() == Some(&key) {
        return;
    }
    let map = regions::regions(civs, &key.0, land, WIDTH, HEIGHT);

    let mut palette = RegionPalette::default();
    palette.colors[usize::from(UNKNOWN)] = swatch(UNKNOWN_COLOUR, 0.30);
    // Neighbouring pieces never share a colour; each keeps its own where it can.
    let colours = map.colouring(palette::REGIONS.len(), |snapshot| palette::civ_index(&civs[snapshot].civ.0));
    for (i, (&snapshot, &colour)) in map.civs.iter().zip(&colours).enumerate() {
        let Some(slot) = palette.colors.get_mut(usize::from(FIRST_CIV) + i) else { break };
        let emphasized = key.1.as_ref().is_none_or(|keep| keep.contains(&civs[snapshot].civ.0));
        *slot = swatch(palette::REGIONS[colour], if emphasized { FILL } else { MASKED });
    }
    if let Some(mut image) = images.get_mut(&civ.labels) {
        image.data = Some(map.labels);
    }
    civ.palette = palette;
    civ.shown = Some(key);
}
