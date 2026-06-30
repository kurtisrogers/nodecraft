use bevy::app::{App, Plugin};
use bevy::asset::load_internal_asset;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, Shader, ShaderRef, SpecializedMeshPipelineError,
};

pub const VOXEL_CHUNK_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x6e6f6465_63726166_63686b00);

/// Minimal vertex-color material for voxel chunks.
/// Avoids the full StandardMaterial PBR shader, which fails on many mobile WebGL2 GPUs.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct VoxelChunkMaterial {
    /// xyz = sun direction, w = daylight factor 0..1
    #[uniform(0)]
    pub sun_dir: Vec4,
}

impl Material for VoxelChunkMaterial {
    fn fragment_shader() -> ShaderRef {
        VOXEL_CHUNK_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

fn load_voxel_chunk_shader(app: &mut App) {
    load_internal_asset!(
        app,
        VOXEL_CHUNK_SHADER_HANDLE,
        "shaders/voxel_chunk.wgsl",
        Shader::from_wgsl
    );
}

pub struct VoxelChunkMaterialPlugin;

impl Plugin for VoxelChunkMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_voxel_chunk_shader(app);
        app.add_plugins(MaterialPlugin::<VoxelChunkMaterial>::default());
    }
}
