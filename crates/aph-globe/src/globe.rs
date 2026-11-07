//! The parchment globe: procedural paper, an analytic graticule, coastlines, and regions.

use bevy::{prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef};

use crate::{
    clock::{self, Clocked, GpuClock},
    overlay::{CivOverlay, RegionPalette},
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GlobeMaterial {
    #[uniform(0)]
    pub clock: GpuClock,
    /// Region labels (see `aph_scene::regions`), read with exact integer loads.
    #[texture(1, sample_type = "u_int")]
    pub labels: Handle<Image>,
    #[uniform(2)]
    pub palette: RegionPalette,
    /// Coastline distance field (see `aph_scene::overlay::coasts`).
    #[texture(3)]
    #[sampler(4)]
    pub coasts: Handle<Image>,
}

impl Material for GlobeMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://aph_globe/shaders/globe.wesl".into()
    }
}

impl Clocked for GlobeMaterial {
    fn clock_mut(&mut self) -> &mut GpuClock {
        &mut self.clock
    }
}

pub struct GlobePlugin;

impl Plugin for GlobePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GlobeMaterial>::default())
            .add_systems(Startup, spawn)
            .add_systems(Update, (clock::sync::<GlobeMaterial>, sync_palette));
    }
}

#[derive(Component)]
struct Globe;

fn spawn(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<GlobeMaterial>>, overlay: Res<CivOverlay>) {
    commands.spawn((
        Globe,
        Name::new("globe"),
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(256, 128))),
        MeshMaterial3d(materials.add(GlobeMaterial {
            clock: GpuClock::default(),
            labels: overlay.labels.clone(),
            palette: overlay.palette,
            coasts: overlay.coasts.clone(),
        })),
    ));
}

fn sync_palette(overlay: Res<CivOverlay>, globe: Query<&MeshMaterial3d<GlobeMaterial>, With<Globe>>, mut materials: ResMut<Assets<GlobeMaterial>>) {
    if !overlay.is_changed() {
        return;
    }
    for handle in &globe {
        if let Some(mut m) = materials.get_mut(&handle.0) {
            m.palette = overlay.palette;
        }
    }
}
