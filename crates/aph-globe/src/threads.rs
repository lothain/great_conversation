//! "Responds to" threads: one batched mesh of screen-space ribbons (see `shaders/threads.wesl`).

use aph_scene::buffers::Ribbons;
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError, VertexFormat},
    shader::ShaderRef,
};

use crate::{
    clock::{self, Clocked, GpuClock},
    scene::Scene,
};

pub const ATTR_PREV: MeshVertexAttribute = MeshVertexAttribute::new("ThreadPrev", 0x7a1e_0001, VertexFormat::Float32x3);
pub const ATTR_NEXT: MeshVertexAttribute = MeshVertexAttribute::new("ThreadNext", 0x7a1e_0002, VertexFormat::Float32x3);
pub const ATTR_LINE: MeshVertexAttribute = MeshVertexAttribute::new("ThreadLine", 0x7a1e_0003, VertexFormat::Float32x4);
pub const ATTR_INK: MeshVertexAttribute = MeshVertexAttribute::new("ThreadInk", 0x7a1e_0004, VertexFormat::Float32x4);
pub const ATTR_END: MeshVertexAttribute = MeshVertexAttribute::new("ThreadEnd", 0x7a1e_0005, VertexFormat::Float32x3);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct ThreadMaterial {
    #[uniform(0)]
    pub clock: GpuClock,
}

impl Clocked for ThreadMaterial {
    fn clock_mut(&mut self) -> &mut GpuClock {
        &mut self.clock
    }
}

impl Material for ThreadMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://aph_globe/shaders/threads.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://aph_globe/shaders/threads.wesl".into()
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
            ATTR_PREV.at_shader_location(1),
            ATTR_NEXT.at_shader_location(2),
            ATTR_LINE.at_shader_location(3),
            ATTR_INK.at_shader_location(4),
            ATTR_END.at_shader_location(5),
        ])?];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[must_use]
pub fn mesh(r: Ribbons) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, r.position)
        .with_inserted_attribute(ATTR_PREV, r.prev)
        .with_inserted_attribute(ATTR_NEXT, r.next)
        .with_inserted_attribute(ATTR_LINE, r.line)
        .with_inserted_attribute(ATTR_INK, r.ink)
        .with_inserted_attribute(ATTR_END, r.end)
        .with_inserted_indices(Indices::U32(r.indices))
}

pub struct ThreadsPlugin;

impl Plugin for ThreadsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ThreadMaterial>::default())
            .add_systems(Startup, spawn)
            .add_systems(Update, clock::sync::<ThreadMaterial>);
    }
}

fn spawn(mut commands: Commands, scene: Res<Scene>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ThreadMaterial>>) {
    commands.spawn((
        Name::new("threads"),
        Mesh3d(meshes.add(mesh(Ribbons::build(&scene.atlas.threads)))),
        MeshMaterial3d(materials.add(ThreadMaterial::default())),
        // Vertices move in the shader; the batch spans the globe anyway.
        NoFrustumCulling,
    ));
}
