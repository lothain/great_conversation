//! Aphorisms and ideas: one batched mesh of screen-space billboards drawn as SDF discs
//! (see `shaders/dots.wesl`), so they stay perfectly round and crisp at any zoom.

use aph_scene::buffers::Dots;
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType, RenderPipelineDescriptor, SpecializedMeshPipelineError, VertexFormat},
    shader::ShaderRef,
};

use crate::{
    clock::{self, Clocked, GpuClock},
    scene::Scene,
};

pub const ATTR_CORNER: MeshVertexAttribute = MeshVertexAttribute::new("DotCorner", 0x7a1e_0101, VertexFormat::Float32x2);
pub const ATTR_PARAMS: MeshVertexAttribute = MeshVertexAttribute::new("DotParams", 0x7a1e_0102, VertexFormat::Float32x4);
pub const ATTR_FILL: MeshVertexAttribute = MeshVertexAttribute::new("DotFill", 0x7a1e_0103, VertexFormat::Float32x4);
pub const ATTR_IDS: MeshVertexAttribute = MeshVertexAttribute::new("DotIds", 0x7a1e_0104, VertexFormat::Float32x2);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct DotMaterial {
    #[uniform(0)]
    pub clock: GpuClock,
    #[uniform(1)]
    pub style: GpuDotStyle,
}

/// Host-controlled emphasis: which religions are at full strength, which node is ringed.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct DotStyle {
    pub religion_mask: Option<u32>,
    pub highlight: Option<u32>,
}

#[derive(ShaderType, Debug, Default, Clone, Copy)]
pub struct GpuDotStyle {
    pub religion_bits: u32,
    pub masked: u32,
    /// Node index to ring, or -1.
    pub highlight: i32,
    pub _pad: u32,
}

impl From<DotStyle> for GpuDotStyle {
    fn from(s: DotStyle) -> Self {
        Self {
            religion_bits: s.religion_mask.unwrap_or(0),
            masked: u32::from(s.religion_mask.is_some()),
            highlight: s.highlight.and_then(|h| i32::try_from(h).ok()).unwrap_or(-1),
            _pad: 0,
        }
    }
}

impl Clocked for DotMaterial {
    fn clock_mut(&mut self) -> &mut GpuClock {
        &mut self.clock
    }
}

impl Material for DotMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://aph_globe/shaders/dots.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://aph_globe/shaders/dots.wesl".into()
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
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTR_CORNER.at_shader_location(1),
            ATTR_PARAMS.at_shader_location(2),
            ATTR_FILL.at_shader_location(3),
            ATTR_IDS.at_shader_location(4),
        ])?];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[must_use]
pub fn mesh(d: Dots) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, d.center)
        .with_inserted_attribute(ATTR_CORNER, d.corner)
        .with_inserted_attribute(ATTR_PARAMS, d.params)
        .with_inserted_attribute(ATTR_FILL, d.fill)
        .with_inserted_attribute(ATTR_IDS, d.ids)
        .with_inserted_indices(Indices::U32(d.indices))
}

pub struct DotsPlugin;

impl Plugin for DotsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<DotMaterial>::default())
            .init_resource::<DotStyle>()
            .add_systems(Startup, spawn)
            .add_systems(Update, (clock::sync::<DotMaterial>, sync_style));
    }
}

fn spawn(mut commands: Commands, scene: Res<Scene>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<DotMaterial>>) {
    commands.spawn((
        Name::new("dots"),
        Mesh3d(meshes.add(mesh(Dots::build(&scene.atlas.nodes)))),
        MeshMaterial3d(materials.add(DotMaterial { clock: GpuClock::default(), style: DotStyle::default().into() })),
        NoFrustumCulling,
    ));
}

fn sync_style(style: Res<DotStyle>, handles: Query<&MeshMaterial3d<DotMaterial>>, mut materials: ResMut<Assets<DotMaterial>>) {
    if !style.is_changed() {
        return;
    }
    for handle in &handles {
        if let Some(mut m) = materials.get_mut(&handle.0) {
            m.style = (*style).into();
        }
    }
}
