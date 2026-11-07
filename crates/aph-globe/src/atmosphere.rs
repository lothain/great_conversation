//! The glow around the globe's limb (see `shaders/atmosphere.wesl`).

use bevy::{prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef};

/// Radius of the glow shell; must match `SHELL` in the shader.
const SHELL: f32 = 1.06;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct AtmosphereMaterial {}

impl Material for AtmosphereMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://aph_globe/shaders/atmosphere.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<AtmosphereMaterial>::default()).add_systems(Startup, spawn);
    }
}

fn spawn(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<AtmosphereMaterial>>) {
    commands.spawn((
        Name::new("atmosphere"),
        Mesh3d(meshes.add(Sphere::new(SHELL).mesh().uv(128, 64))),
        MeshMaterial3d(materials.add(AtmosphereMaterial::default())),
    ));
}
