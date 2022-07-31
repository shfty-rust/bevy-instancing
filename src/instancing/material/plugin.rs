use crate::prelude::{DrawIndexedIndirect, DrawIndirect};
use bevy::{
    app::{App, Plugin},
    asset::AddAsset,
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
    ecs::{
        component::TableStorage,
        system::{
            lifetimeless::{Read, SQuery, SRes},
            SystemParamItem,
        },
    },
    pbr::{AlphaMode, SetMeshViewBindGroup},
    prelude::{
        debug, default, info, Deref, DerefMut, Entity, Handle, Mesh,
        ParallelSystemDescriptorCoercion,
    },
    render::{
        extract_component::ExtractComponentPlugin,
        mesh::{Indices, MeshVertexBufferLayout, PrimitiveTopology},
        render_asset::{PrepareAssetLabel, RenderAssetPlugin},
        render_phase::{
            AddRenderCommand, EntityRenderCommand, RenderCommandResult, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::{
            BindingResource, BufferBindingType, BufferVec, DynamicUniformBuffer, IndexFormat,
            ShaderType, SpecializedMeshPipelines, StorageBuffer,
        },
        renderer::RenderQueue,
        RenderApp, RenderStage,
    },
};
use bevy::{
    prelude::Component,
    render::{
        render_resource::{BindGroup, Buffer},
        renderer::RenderDevice,
    },
};

use crate::prelude::{
    extract_mesh_instances, Instance, InstanceBlockRange, InstancedMaterialPipeline,
    SetInstancedMaterialBindGroup, SetInstancedMeshBindGroup, SpecializedInstancedMaterial,
};

use std::collections::{BTreeMap, BTreeSet};

use std::marker::PhantomData;

use super::systems::{
    extract_instanced_meshes, prepare_batched_instances, prepare_instance_batches,
    prepare_instanced_view_meta, prepare_material_batches, prepare_mesh_batches,
    prepare_view_instance_blocks, prepare_view_instances, queue_instanced_materials,
};

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`SpecializedMaterial`]
/// asset type (which includes [`Material`] types).
pub struct InstancedMaterialPlugin<M: SpecializedInstancedMaterial>(PhantomData<M>);

impl<M: SpecializedInstancedMaterial> Default for InstancedMaterialPlugin<M> {
    fn default() -> Self {
        Self(default())
    }
}

impl<M: SpecializedInstancedMaterial> Plugin for InstancedMaterialPlugin<M>
where
    <M::Instance as Instance>::PreparedInstance: ShaderType,
{
    fn build(&self, app: &mut App) {
        app.add_asset::<M>()
            .add_plugin(ExtractComponentPlugin::<Handle<M>>::default())
            .add_plugin(ExtractComponentPlugin::<Handle<Mesh>>::default())
            .add_plugin(RenderAssetPlugin::<M>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawInstanced<M>>()
                .add_render_command::<Opaque3d, DrawInstanced<M>>()
                .add_render_command::<AlphaMask3d, DrawInstanced<M>>()
                .init_resource::<InstanceViewMeta<M>>()
                .init_resource::<InstancedMaterialPipeline<M>>()
                .init_resource::<SpecializedMeshPipelines<InstancedMaterialPipeline<M>>>()
                .add_system_to_stage(RenderStage::Extract, extract_mesh_instances::<M>)
                .add_system_to_stage(RenderStage::Extract, extract_instanced_meshes::system::<M>)
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_instanced_view_meta::system::<M>
                        .before(prepare_view_instances::system::<M>)
                        .before(prepare_view_instance_blocks::system::<M>),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_view_instances::system::<M>.before(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_view_instance_blocks::system::<M>
                        .before(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_mesh_batches::system::<M>.after(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_material_batches::system::<M>.after(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_instance_batches::system::<M>
                        .after(prepare_mesh_batches::system::<M>)
                        .after(prepare_material_batches::system::<M>),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_batched_instances::system::<M>
                        .after(prepare_instance_batches::system::<M>),
                )
                .add_system_to_stage(RenderStage::Queue, queue_instanced_materials::system::<M>);
        }
    }
}

/// Unique key describing a set of mutually incompatible meshes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstancedMeshKey {
    pub primitive_topology: PrimitiveTopology,
    pub layout: MeshVertexBufferLayout,
    pub index_format: Option<IndexFormat>,
}

impl PartialOrd for InstancedMeshKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.primitive_topology as usize).partial_cmp(&(other.primitive_topology as usize)) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.index_format
            .map(|index_format| index_format as usize)
            .partial_cmp(&other.index_format.map(|index_format| index_format as usize))
    }
}

impl Ord for InstancedMeshKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.primitive_topology as usize).cmp(&(other.primitive_topology as usize)) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.index_format
            .map(|index_format| index_format as usize)
            .cmp(&other.index_format.map(|index_format| index_format as usize))
    }
}

/// Render world representation of an instanced mesh
#[derive(Debug, Clone)]
pub struct GpuInstancedMesh {
    pub vertex_buffer_data: Vec<u8>,
    pub vertex_count: usize,
    pub index_buffer_data: GpuIndexBufferData,
    pub primitive_topology: PrimitiveTopology,
    pub layout: MeshVertexBufferLayout,
    pub key: InstancedMeshKey,
}

#[derive(Debug, Clone)]
pub enum GpuIndexBufferData {
    Indexed {
        indices: Indices,
        index_count: u32,
        index_format: IndexFormat,
    },
    NonIndexed {
        vertex_count: u32,
    },
}

#[derive(Debug, Clone)]
pub struct GpuInstancedMeshes<M: SpecializedInstancedMaterial> {
    pub instanced_meshes: BTreeMap<Handle<Mesh>, GpuInstancedMesh>,
    pub _phantom: PhantomData<M>,
}

impl<M: SpecializedInstancedMaterial> Default for GpuInstancedMeshes<M> {
    fn default() -> Self {
        GpuInstancedMeshes {
            instanced_meshes: default(),
            _phantom: default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum GpuIndirectData {
    NonIndexed { buffer: Vec<DrawIndirect> },
    Indexed { buffer: Vec<DrawIndexedIndirect> },
}

impl GpuIndirectData {
    pub fn len(&self) -> usize {
        match self {
            GpuIndirectData::NonIndexed { buffer } => buffer.len(),
            GpuIndirectData::Indexed { buffer } => buffer.len(),
        }
    }
}

/// Key-friendly equivalent of AlphaMode
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpuAlphaMode {
    Opaque,
    Mask,
    Blend,
}

impl From<AlphaMode> for GpuAlphaMode {
    fn from(alpha_mode: AlphaMode) -> Self {
        match alpha_mode {
            AlphaMode::Opaque => GpuAlphaMode::Opaque,
            AlphaMode::Mask(_) => GpuAlphaMode::Mask,
            AlphaMode::Blend => GpuAlphaMode::Blend,
        }
    }
}

/// Unique key describing a set of mutually incompatible materials
pub struct InstancedMaterialBatchKey<M: SpecializedInstancedMaterial> {
    pub alpha_mode: GpuAlphaMode,
    pub key: M::BatchKey,
}

impl<M: SpecializedInstancedMaterial> Clone for InstancedMaterialBatchKey<M> {
    fn clone(&self) -> Self {
        Self {
            alpha_mode: self.alpha_mode.clone(),
            key: self.key.clone(),
        }
    }
}

impl<M: SpecializedInstancedMaterial> PartialEq for InstancedMaterialBatchKey<M> {
    fn eq(&self, other: &Self) -> bool {
        self.alpha_mode == other.alpha_mode && self.key == other.key
    }
}

impl<M: SpecializedInstancedMaterial> Eq for InstancedMaterialBatchKey<M> {}

impl<M: SpecializedInstancedMaterial> PartialOrd for InstancedMaterialBatchKey<M>
where
    M::BatchKey: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.alpha_mode.partial_cmp(&other.alpha_mode) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.key.partial_cmp(&other.key)
    }
}

impl<M: SpecializedInstancedMaterial> Ord for InstancedMaterialBatchKey<M>
where
    M::BatchKey: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.alpha_mode.cmp(&other.alpha_mode) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.key.cmp(&other.key)
    }
}

impl<M: SpecializedInstancedMaterial> std::fmt::Debug for InstancedMaterialBatchKey<M>
where
    M::BatchKey: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstancedMaterialKey")
            .field("alpha_mode", &self.alpha_mode)
            .field("key", &self.key)
            .finish()
    }
}

/// Unique key describing a set of mutually incompatible instances
pub struct InstanceBatchKey<M: SpecializedInstancedMaterial> {
    pub mesh_key: InstancedMeshKey,
    pub material_key: InstancedMaterialBatchKey<M>,
}

impl<M: SpecializedInstancedMaterial> Component for InstanceBatchKey<M> {
    type Storage = TableStorage;
}

impl<M> Clone for InstanceBatchKey<M>
where
    M: SpecializedInstancedMaterial,
{
    fn clone(&self) -> Self {
        Self {
            mesh_key: self.mesh_key.clone(),
            material_key: self.material_key.clone(),
        }
    }
}

impl<M: SpecializedInstancedMaterial> PartialEq for InstanceBatchKey<M> {
    fn eq(&self, other: &Self) -> bool {
        self.mesh_key == other.mesh_key && self.material_key == other.material_key
    }
}

impl<M: SpecializedInstancedMaterial> Eq for InstanceBatchKey<M> {}

impl<M: SpecializedInstancedMaterial> PartialOrd for InstanceBatchKey<M>
where
    M::BatchKey: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.mesh_key.partial_cmp(&other.mesh_key) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.material_key.partial_cmp(&other.material_key)
    }
}

impl<M: SpecializedInstancedMaterial> Ord for InstanceBatchKey<M>
where
    M::BatchKey: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.mesh_key.cmp(&other.mesh_key) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.material_key.cmp(&other.material_key)
    }
}

impl<M: SpecializedInstancedMaterial> std::fmt::Debug for InstanceBatchKey<M>
where
    M::BatchKey: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceKey")
            .field("mesh_key", &self.mesh_key)
            .field("material_key", &self.material_key)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct MeshBatch {
    pub meshes: BTreeSet<Handle<Mesh>>,
    pub vertex_data: Vec<u8>,
    pub index_data: Option<GpuIndexBufferData>,
    pub indirect_data: GpuIndirectData,
}

pub const MAX_UNIFORM_BUFFER_INSTANCES: usize = 112;

pub enum GpuInstances<M: SpecializedInstancedMaterial> {
    Uniform {
        buffer: DynamicUniformBuffer<
            [<M::Instance as Instance>::PreparedInstance; MAX_UNIFORM_BUFFER_INSTANCES],
        >,
    },
    Storage {
        buffer: StorageBuffer<Vec<<M::Instance as Instance>::PreparedInstance>>,
    },
}

impl<M: SpecializedInstancedMaterial> GpuInstances<M> {
    pub fn new(buffer_binding_type: BufferBindingType) -> Self {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => Self::storage(),
            BufferBindingType::Uniform => Self::uniform(),
        }
    }

    pub fn uniform() -> Self {
        Self::Uniform {
            buffer: DynamicUniformBuffer::default(),
        }
    }

    pub fn storage() -> Self {
        Self::Storage {
            buffer: StorageBuffer::default(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            Self::Uniform { buffer } => buffer.clear(),
            Self::Storage { buffer } => buffer.get_mut().clear(),
        }
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        match self {
            Self::Uniform { buffer } => buffer.buffer(),
            Self::Storage { buffer } => buffer.buffer(),
        }
    }

    pub fn push(&mut self, mut instances: Vec<<M::Instance as Instance>::PreparedInstance>) {
        match self {
            Self::Uniform { buffer } => {
                // NOTE: This iterator construction allows moving and padding with default
                // values and is like this to avoid unnecessary cloning.
                let gpu_instances = instances
                    .drain(..)
                    .chain(std::iter::repeat_with(default))
                    .take(MAX_UNIFORM_BUFFER_INSTANCES)
                    .collect::<Vec<_>>();
                let gpu_instances = gpu_instances.try_into().unwrap();
                buffer.push(gpu_instances);
            }
            Self::Storage { buffer } => {
                for instance in instances {
                    buffer.get_mut().push(instance);
                }
            }
        }
    }

    pub fn write_buffer(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        match self {
            Self::Uniform { buffer } => buffer.write_buffer(render_device, render_queue),
            Self::Storage { buffer } => buffer.write_buffer(render_device, render_queue),
        }
    }

    pub fn binding(&self) -> Option<BindingResource> {
        match self {
            Self::Uniform { buffer } => buffer.binding(),
            Self::Storage { buffer } => buffer.binding(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Uniform { buffer } => buffer.len(),
            Self::Storage { buffer } => buffer.get().len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct InstanceBatch<M: SpecializedInstancedMaterial> {
    pub instances: BTreeSet<Entity>,
    pub instance_block_ranges: BTreeMap<Entity, InstanceBlockRange>,
    pub instance_buffer_data: GpuInstances<M>,
}

#[derive(Deref, DerefMut)]
pub struct InstanceViewMeta<M: SpecializedInstancedMaterial> {
    pub view_meta: BTreeMap<Entity, InstanceMeta<M>>,
}

impl<M: SpecializedInstancedMaterial> Default for InstanceViewMeta<M> {
    fn default() -> Self {
        Self {
            view_meta: default(),
        }
    }
}

pub struct MaterialBatch<M: SpecializedInstancedMaterial> {
    pub material: Handle<M>,
    pub pipeline_key: M::PipelineKey,
}

impl<M: SpecializedInstancedMaterial> std::fmt::Debug for MaterialBatch<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaterialBatch")
            .field("material", &self.material)
            .field("pipeline_key", &self.pipeline_key)
            .finish()
    }
}

/// Resource containing instance batches
pub struct InstanceMeta<M: SpecializedInstancedMaterial> {
    pub instances: Vec<Entity>,
    pub instance_blocks: Vec<Entity>,
    pub mesh_batches: BTreeMap<InstancedMeshKey, MeshBatch>,
    pub material_batches: BTreeMap<InstancedMaterialBatchKey<M>, MaterialBatch<M>>,
    pub instance_batches: BTreeMap<InstanceBatchKey<M>, InstanceBatch<M>>,
    pub batched_instances: BTreeMap<InstanceBatchKey<M>, BatchedInstances>,
}

impl<M: SpecializedInstancedMaterial> Default for InstanceMeta<M> {
    fn default() -> Self {
        Self {
            instances: default(),
            instance_blocks: default(),
            mesh_batches: default(),
            material_batches: default(),
            instance_batches: default(),
            batched_instances: default(),
        }
    }
}

pub enum GpuIndirectBufferData {
    Indexed {
        indirects: Vec<DrawIndexedIndirect>,
        buffer: BufferVec<DrawIndexedIndirect>,
    },
    NonIndexed {
        indirects: Vec<DrawIndirect>,
        buffer: BufferVec<DrawIndirect>,
    },
}

impl GpuIndirectBufferData {
    fn buffer(&self) -> Option<&Buffer> {
        match self {
            GpuIndirectBufferData::Indexed { buffer, .. } => buffer.buffer(),
            GpuIndirectBufferData::NonIndexed { buffer, .. } => buffer.buffer(),
        }
    }

    fn indirects(&self) -> Option<&Vec<DrawIndirect>> {
        match self {
            GpuIndirectBufferData::NonIndexed { indirects, .. } => Some(indirects),
            _ => None,
        }
    }

    fn indexed_indirects(&self) -> Option<&Vec<DrawIndexedIndirect>> {
        match self {
            GpuIndirectBufferData::Indexed { indirects, .. } => Some(indirects),
            _ => None,
        }
    }
}

/// The data necessary to render one set of mutually compatible instances
#[derive(Component)]
pub struct BatchedInstances {
    pub view_entity: Entity,
    pub batch_entity: Entity,
    pub vertex_buffer: Buffer,
    pub index_data: Option<(Buffer, IndexFormat)>,
    pub indirect_buffer: GpuIndirectBufferData,
    pub instance_bind_group: BindGroup,
    pub indirect_count: usize,
    pub indirect_indices: Vec<usize>,
}

pub type DrawInstanced<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetInstancedMaterialBindGroup<M, 1>,
    SetInstancedMeshBindGroup<M, 2>,
    DrawBatchedInstances<M>,
);

/// Render command for drawing instanced meshes
pub struct DrawBatchedInstances<M: SpecializedInstancedMaterial>(PhantomData<M>);

impl<M: SpecializedInstancedMaterial> EntityRenderCommand for DrawBatchedInstances<M> {
    type Param = (
        SRes<RenderDevice>,
        SRes<InstanceViewMeta<M>>,
        SQuery<Read<InstanceBatchKey<M>>>,
    );
    #[inline]
    fn render<'w>(
        view: Entity,
        item: Entity,
        (render_device, instance_meta, query_instance_batch_key): SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        debug!("DrawInstanceBatch {item:?}");
        let batched_instances = instance_meta
            .into_inner()
            .get(&view)
            .unwrap()
            .batched_instances
            .get(query_instance_batch_key.get(item).unwrap())
            .unwrap();

        pass.set_vertex_buffer(0, batched_instances.vertex_buffer.slice(..));

        match &batched_instances.index_data {
            Some((index_buffer, index_format)) => {
                pass.set_index_buffer(index_buffer.slice(..), 0, *index_format);

                if render_device
                    .features()
                    .contains(wgpu::Features::INDIRECT_FIRST_INSTANCE)
                {
                    for i in &batched_instances.indirect_indices {
                        let indirect = batched_instances
                            .indirect_buffer
                            .indexed_indirects()
                            .unwrap()[*i];

                        info!("Drawing indexed indirect {i:?}: {indirect:#?}");

                        pass.draw_indexed_indirect(
                            batched_instances.indirect_buffer.buffer().unwrap(),
                            (i * std::mem::size_of::<DrawIndexedIndirect>()) as u64,
                        );
                    }
                } else {
                    for i in &batched_instances.indirect_indices {
                        let indirect = batched_instances
                            .indirect_buffer
                            .indexed_indirects()
                            .unwrap()[*i];

                        info!("Drawing indexed direct {i:?}: {indirect:#?}");

                        let DrawIndexedIndirect {
                            vertex_count,
                            instance_count,
                            base_index,
                            vertex_offset,
                            base_instance,
                        } = indirect;

                        pass.draw_indexed(
                            base_index..(base_index + vertex_count),
                            vertex_offset,
                            base_instance..base_instance + instance_count,
                        );
                    }
                }
            }
            None => {
                if render_device
                    .features()
                    .contains(wgpu::Features::INDIRECT_FIRST_INSTANCE)
                {
                    for i in &batched_instances.indirect_indices {
                        let indirect = batched_instances.indirect_buffer.indirects().unwrap()[*i];
                        debug!("Drawing indirect {i:?}: {indirect:#?}");

                        pass.draw_indirect(
                            batched_instances.indirect_buffer.buffer().unwrap(),
                            (i * std::mem::size_of::<DrawIndirect>()) as u64,
                        );
                    }
                } else {
                    for i in &batched_instances.indirect_indices {
                        let indirect = batched_instances.indirect_buffer.indirects().unwrap()[*i];
                        info!("Drawing direct {i:?}: {indirect:#?}");

                        let DrawIndirect {
                            vertex_count,
                            instance_count,
                            base_vertex,
                            base_instance,
                        } = indirect;

                        pass.draw(
                            base_vertex..(base_vertex + vertex_count),
                            base_instance..(base_instance + instance_count),
                        );
                    }
                }
            }
        }

        RenderCommandResult::Success
    }
}
