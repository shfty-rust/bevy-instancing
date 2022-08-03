use crate::{
    instancing::{mesh_instance::MeshInstance, render::instance::InstanceUniformLength},
    prelude::{DrawIndexedIndirect, DrawIndirect},
};
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
        debug, default, info, AssetEvent, Assets, Commands, Deref, DerefMut, Entity, EventReader,
        Handle, Image, Local, Mesh, ParallelSystemDescriptorCoercion, Res, ResMut,
    },
    render::{
        extract_component::ExtractComponentPlugin,
        mesh::{Indices, MeshVertexBufferLayout, PrimitiveTopology},
        render_asset::{PrepareAssetLabel, RenderAssets},
        render_phase::{
            AddRenderCommand, EntityRenderCommand, RenderCommandResult, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::{
            AsBindGroupError, BufferBindingType, IndexFormat, OwnedBindingResource, ShaderType,
            SpecializedMeshPipelines, StorageBuffer, UniformBuffer,
        },
        renderer::RenderQueue,
        texture::FallbackImage,
        Extract, RenderApp, RenderStage,
    },
    utils::{HashMap, HashSet},
};
use bevy::{
    prelude::Component,
    render::{
        render_resource::{BindGroup, Buffer},
        renderer::RenderDevice,
    },
};

use crate::prelude::{
    extract_mesh_instances, Instance, InstanceSliceRange, InstancedMaterialPipeline,
    MaterialInstanced, SetInstancedMaterialBindGroup,
};

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    hash::Hash,
};

use std::marker::PhantomData;

use super::systems::{
    extract_instanced_meshes, extract_instanced_view_meta, prepare_batched_instances::{self, ViewIndirectData},
    prepare_instance_batches::{self, ViewInstanceData},
    prepare_instance_slice_targets,
    prepare_material_batches::{self, MaterialBatches},
    prepare_mesh_batches, prepare_view_instance_slices, prepare_view_instances,
    queue_instanced_materials,
};

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`SpecializedMaterial`]
/// asset type (which includes [`Material`] types).
pub struct InstancedMaterialPlugin<M: MaterialInstanced>(PhantomData<M>);

impl<M: MaterialInstanced> Default for InstancedMaterialPlugin<M> {
    fn default() -> Self {
        Self(default())
    }
}

impl<M: MaterialInstanced> Plugin for InstancedMaterialPlugin<M>
where
    M::Data: Debug + Clone + Hash + PartialEq + Eq,
    <M::Instance as Instance>::PreparedInstance: ShaderType,
{
    fn build(&self, app: &mut App) {
        app.add_asset::<M>()
            .add_plugin(ExtractComponentPlugin::<Handle<M>>::default())
            .add_plugin(ExtractComponentPlugin::<Handle<Mesh>>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawInstanced<M>>()
                .add_render_command::<Opaque3d, DrawInstanced<M>>()
                .add_render_command::<AlphaMask3d, DrawInstanced<M>>()
                .init_resource::<InstancedMaterialPipeline<M>>()
                .init_resource::<ExtractedMaterials<M>>()
                .init_resource::<RenderMeshes>()
                .init_resource::<RenderMaterials<M>>()
                .init_resource::<MaterialBatches<M>>()
                .init_resource::<MaterialBatches<M>>()
                .init_resource::<ViewInstanceData<M>>()
                .init_resource::<ViewIndirectData<M>>()
                .init_resource::<SpecializedMeshPipelines<InstancedMaterialPipeline<M>>>()
                .add_system_to_stage(RenderStage::Extract, extract_materials::<M>)
                .add_system_to_stage(RenderStage::Extract, extract_mesh_instances::<M>)
                .add_system_to_stage(RenderStage::Extract, extract_instanced_meshes::system)
                .add_system_to_stage(
                    RenderStage::Extract,
                    extract_instanced_view_meta::system::<M>,
                )
                .add_system_to_stage(RenderStage::Prepare, prepare_materials::<M>)
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_view_instances::system::<M>.before(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_view_instance_slices::system::<M>
                        .before(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_material_batches::system::<M>.after(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_instance_batches::system::<M>
                        .after(prepare_mesh_batches::system)
                        .after(prepare_material_batches::system::<M>),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_batched_instances::system::<M>
                        .after(prepare_instance_batches::system::<M>),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_instance_slice_targets::system::<M>
                        .after(prepare_batched_instances::system::<M>),
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

#[derive(Debug, Clone)]
pub enum GpuIndexBufferData {
    Indexed {
        indices: Indices,
        index_format: IndexFormat,
    },
    NonIndexed {
        vertex_count: u32,
    },
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

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct RenderMeshes {
    pub instanced_meshes: BTreeMap<Handle<Mesh>, GpuInstancedMesh>,
}

impl Default for RenderMeshes {
    fn default() -> Self {
        RenderMeshes {
            instanced_meshes: default(),
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
pub struct InstancedMaterialBatchKey<M: MaterialInstanced> {
    pub alpha_mode: GpuAlphaMode,
    pub key: M::BatchKey,
}

impl<M: MaterialInstanced> Clone for InstancedMaterialBatchKey<M> {
    fn clone(&self) -> Self {
        Self {
            alpha_mode: self.alpha_mode.clone(),
            key: self.key.clone(),
        }
    }
}

impl<M: MaterialInstanced> PartialEq for InstancedMaterialBatchKey<M> {
    fn eq(&self, other: &Self) -> bool {
        self.alpha_mode == other.alpha_mode && self.key == other.key
    }
}

impl<M: MaterialInstanced> Eq for InstancedMaterialBatchKey<M> {}

impl<M: MaterialInstanced> PartialOrd for InstancedMaterialBatchKey<M>
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

impl<M: MaterialInstanced> Ord for InstancedMaterialBatchKey<M>
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

impl<M: MaterialInstanced> Debug for InstancedMaterialBatchKey<M>
where
    M::BatchKey: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstancedMaterialKey")
            .field("alpha_mode", &self.alpha_mode)
            .field("key", &self.key)
            .finish()
    }
}

/// Unique key describing a set of mutually incompatible instances
pub struct InstanceBatchKey<M: MaterialInstanced> {
    pub mesh_key: InstancedMeshKey,
    pub material_key: InstancedMaterialBatchKey<M>,
}

impl<M: MaterialInstanced> Component for InstanceBatchKey<M> {
    type Storage = TableStorage;
}

impl<M> Clone for InstanceBatchKey<M>
where
    M: MaterialInstanced,
{
    fn clone(&self) -> Self {
        Self {
            mesh_key: self.mesh_key.clone(),
            material_key: self.material_key.clone(),
        }
    }
}

impl<M: MaterialInstanced> PartialEq for InstanceBatchKey<M> {
    fn eq(&self, other: &Self) -> bool {
        self.mesh_key == other.mesh_key && self.material_key == other.material_key
    }
}

impl<M: MaterialInstanced> Eq for InstanceBatchKey<M> {}

impl<M: MaterialInstanced> PartialOrd for InstanceBatchKey<M>
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

impl<M: MaterialInstanced> Ord for InstanceBatchKey<M>
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

impl<M: MaterialInstanced> Debug for InstanceBatchKey<M>
where
    M::BatchKey: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceKey")
            .field("mesh_key", &self.mesh_key)
            .field("material_key", &self.material_key)
            .finish()
    }
}

const MAX_UNIFORM_BUFFER_LENGTH: usize = MeshInstance::UNIFORM_BUFFER_LENGTH.get() as usize;

pub enum GpuInstances<M: MaterialInstanced> {
    Uniform {
        buffers: Vec<
            UniformBuffer<[<M::Instance as Instance>::PreparedInstance; MAX_UNIFORM_BUFFER_LENGTH]>,
        >,
    },
    Storage {
        buffer: StorageBuffer<Vec<<M::Instance as Instance>::PreparedInstance>>,
    },
}

impl<M: MaterialInstanced> GpuInstances<M> {
    pub fn new(buffer_binding_type: BufferBindingType) -> Self {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => Self::storage(),
            BufferBindingType::Uniform => Self::uniform(),
        }
    }

    pub fn uniform() -> Self {
        Self::Uniform { buffers: default() }
    }

    pub fn storage() -> Self {
        Self::Storage {
            buffer: StorageBuffer::default(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            Self::Uniform { buffers } => buffers.clear(),
            Self::Storage { buffer } => buffer.get_mut().clear(),
        }
    }

    pub fn set(&mut self, instances: Vec<<M::Instance as Instance>::PreparedInstance>) {
        self.clear();

        match self {
            Self::Uniform { buffers } => {
                for chunk in instances.chunks(
                    <M::Instance as InstanceUniformLength>::UNIFORM_BUFFER_LENGTH.get() as usize,
                ) {
                    let mut buf: [<M::Instance as Instance>::PreparedInstance;
                        MAX_UNIFORM_BUFFER_LENGTH] = vec![
                            <M::Instance as Instance>::PreparedInstance::default();
                            MAX_UNIFORM_BUFFER_LENGTH
                        ]
                    .try_into()
                    .unwrap();

                    for (i, instance) in chunk.into_iter().enumerate() {
                        buf[i] = instance.clone();
                    }

                    let buf = UniformBuffer::from(buf);

                    buffers.push(buf);
                }
            }
            Self::Storage { buffer } => {
                buffer.get_mut().extend(instances);
            }
        }
    }

    pub fn write_buffer(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        match self {
            Self::Uniform { buffers } => {
                for buffer in buffers {
                    buffer.write_buffer(render_device, render_queue)
                }
            }
            Self::Storage { buffer } => buffer.write_buffer(render_device, render_queue),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Uniform { .. } => 128,
            Self::Storage { buffer } => buffer.get().len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct InstanceBatch<M: MaterialInstanced> {
    pub instances: BTreeSet<Entity>,
    pub instance_slice_ranges: BTreeMap<Entity, InstanceSliceRange>,
    pub _phantom: PhantomData<M>,
}

impl<M: MaterialInstanced> Debug for InstanceBatch<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceBatch")
            .field("instances", &self.instances)
            .field("instance_slice_ranges", &self.instance_slice_ranges)
            .finish()
    }
}

pub struct MaterialBatch<M: MaterialInstanced> {
    pub material: Handle<M>,
    pub pipeline_key: M::Data,
}

impl<M: MaterialInstanced> Debug for MaterialBatch<M>
where
    M::Data: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaterialBatch")
            .field("material", &self.material)
            .field("pipeline_key", &self.pipeline_key)
            .finish()
    }
}

/// Resource containing per-view instance data
#[derive(Component)]
pub struct InstanceMeta<M: MaterialInstanced> {
    pub instances: Vec<Entity>,
    pub instance_slices: Vec<Entity>,
    pub instance_batches: BTreeMap<InstanceBatchKey<M>, InstanceBatch<M>>,
    pub batched_instances: BTreeMap<InstanceBatchKey<M>, Vec<BatchedInstances>>,
}

impl<M: MaterialInstanced> Default for InstanceMeta<M> {
    fn default() -> Self {
        Self {
            instances: default(),
            instance_slices: default(),
            instance_batches: default(),
            batched_instances: default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum GpuIndirectBufferData {
    Indexed {
        indirects: Vec<DrawIndexedIndirect>,
        buffer: Buffer,
    },
    NonIndexed {
        indirects: Vec<DrawIndirect>,
        buffer: Buffer,
    },
}

impl GpuIndirectBufferData {
    pub fn buffer(&self) -> &Buffer {
        match self {
            GpuIndirectBufferData::Indexed { buffer, .. } => buffer,
            GpuIndirectBufferData::NonIndexed { buffer, .. } => buffer,
        }
    }

    pub fn indirects(&self) -> Option<&Vec<DrawIndirect>> {
        match self {
            GpuIndirectBufferData::NonIndexed { indirects, .. } => Some(indirects),
            _ => None,
        }
    }

    pub fn indexed_indirects(&self) -> Option<&Vec<DrawIndexedIndirect>> {
        match self {
            GpuIndirectBufferData::Indexed { indirects, .. } => Some(indirects),
            _ => None,
        }
    }
}

/// The data necessary to render one set of mutually compatible instances
#[derive(Component)]
pub struct BatchedInstances {
    pub vertex_buffer: Buffer,
    pub index_buffer: Option<(Buffer, IndexFormat)>,
    pub indirect_buffer: GpuIndirectBufferData,
    pub bind_group: BindGroup,
}

pub type DrawInstanced<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetInstancedMaterialBindGroup<M, 1>,
    DrawBatchedInstances<M>,
);

/// Render command for drawing instanced meshes
pub struct DrawBatchedInstances<M: MaterialInstanced>(PhantomData<M>);

impl<M: MaterialInstanced> EntityRenderCommand for DrawBatchedInstances<M> {
    type Param = (
        SRes<RenderDevice>,
        SQuery<Read<InstanceMeta<M>>>,
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
            .get_inner(view)
            .unwrap()
            .batched_instances
            .get(query_instance_batch_key.get(item).unwrap())
            .unwrap();

        for (i, batch) in batched_instances.into_iter().enumerate() {
            pass.set_bind_group(2, &batch.bind_group, &[]);

            pass.set_vertex_buffer(0, batch.vertex_buffer.slice(..));

            match &batch.index_buffer {
                Some((index_buffer, index_format)) => {
                    pass.set_index_buffer(index_buffer.slice(..), 0, *index_format);

                    for (i, indirect) in batch
                        .indirect_buffer
                        .indexed_indirects()
                        .unwrap()
                        .iter()
                        .enumerate()
                    {
                        if render_device
                            .features()
                            .contains(wgpu::Features::INDIRECT_FIRST_INSTANCE)
                        {
                            debug!("Drawing indexed indirect {i:?}: {indirect:#?}");

                            pass.draw_indexed_indirect(
                                batch.indirect_buffer.buffer(),
                                (i * std::mem::size_of::<DrawIndexedIndirect>()) as u64,
                            );
                        } else {
                            debug!("Drawing indexed direct {i:?}: {indirect:#?}");

                            let DrawIndexedIndirect {
                                vertex_count,
                                instance_count,
                                base_index,
                                vertex_offset,
                                base_instance,
                            } = *indirect;

                            pass.draw_indexed(
                                base_index..base_index + vertex_count,
                                vertex_offset,
                                base_instance..base_instance + instance_count,
                            );
                        }
                    }
                }
                None => {
                    for (i, indirect) in batch
                        .indirect_buffer
                        .indirects()
                        .unwrap()
                        .iter()
                        .enumerate()
                    {
                        if render_device
                            .features()
                            .contains(wgpu::Features::INDIRECT_FIRST_INSTANCE)
                        {
                            debug!("Drawing indirect {i:?}: {indirect:#?}");

                            pass.draw_indirect(
                                batch.indirect_buffer.buffer(),
                                (i * std::mem::size_of::<DrawIndirect>()) as u64,
                            );
                        } else {
                            info!("Drawing direct {i:?}: {indirect:#?}");

                            let DrawIndirect {
                                vertex_count,
                                instance_count,
                                base_vertex,
                                base_instance,
                            } = *indirect;

                            pass.draw(
                                base_vertex..base_vertex + vertex_count,
                                base_instance..base_instance + instance_count,
                            );
                        }
                    }
                }
            }
        }

        RenderCommandResult::Success
    }
}

/// Common [`Material`] properties, calculated for a specific material instance.
pub struct MaterialProperties {
    /// The [`AlphaMode`] of this material.
    pub alpha_mode: AlphaMode,
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with equal depth, to avoid z-fighting.
    pub depth_bias: f32,
}

/// Data prepared for a [`Material`] instance.
pub struct PreparedMaterial<T: MaterialInstanced> {
    pub bindings: Vec<OwnedBindingResource>,
    pub bind_group: BindGroup,
    pub pipeline_key: T::Data,
    pub batch_key: T::BatchKey,
    pub properties: MaterialProperties,
}

struct ExtractedMaterials<M: MaterialInstanced> {
    extracted: Vec<(Handle<M>, M)>,
    removed: Vec<Handle<M>>,
}

impl<M: MaterialInstanced> Default for ExtractedMaterials<M> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

/// Stores all prepared representations of [`Material`] assets for as long as they exist.
pub type RenderMaterials<T> = HashMap<Handle<T>, PreparedMaterial<T>>;

/// This system extracts all created or modified assets of the corresponding [`Material`] type
/// into the "render world".
fn extract_materials<M: MaterialInstanced>(
    mut commands: Commands,
    mut events: Extract<EventReader<AssetEvent<M>>>,
    assets: Extract<Res<Assets<M>>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                changed_assets.insert(handle.clone_weak());
            }
            AssetEvent::Removed { handle } => {
                changed_assets.remove(handle);
                removed.push(handle.clone_weak());
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for handle in changed_assets.drain() {
        if let Some(asset) = assets.get(&handle) {
            extracted_assets.push((handle, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedMaterials {
        extracted: extracted_assets,
        removed,
    });
}

/// All [`Material`] values of a given type that should be prepared next frame.
pub struct PrepareNextFrameMaterials<M: MaterialInstanced> {
    assets: Vec<(Handle<M>, M)>,
}

impl<M: MaterialInstanced> Default for PrepareNextFrameMaterials<M> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

/// This system prepares all assets of the corresponding [`Material`] type
/// which where extracted this frame for the GPU.
fn prepare_materials<M: MaterialInstanced>(
    mut prepare_next_frame: Local<PrepareNextFrameMaterials<M>>,
    mut extracted_assets: ResMut<ExtractedMaterials<M>>,
    mut render_materials: ResMut<RenderMaterials<M>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    pipeline: Res<InstancedMaterialPipeline<M>>,
) {
    let mut queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (handle, material) in queued_assets.drain(..) {
        match prepare_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_assets.removed) {
        render_materials.remove(&removed);
    }

    for (handle, material) in std::mem::take(&mut extracted_assets.extracted) {
        match prepare_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }
}

fn prepare_material<M: MaterialInstanced>(
    material: &M,
    render_device: &RenderDevice,
    images: &RenderAssets<Image>,
    fallback_image: &FallbackImage,
    pipeline: &InstancedMaterialPipeline<M>,
) -> Result<PreparedMaterial<M>, AsBindGroupError> {
    let prepared = material.as_bind_group(
        &pipeline.material_layout,
        render_device,
        images,
        fallback_image,
    )?;
    Ok(PreparedMaterial {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        pipeline_key: prepared.data,
        batch_key: M::BatchKey::from(material),
        properties: MaterialProperties {
            alpha_mode: material.alpha_mode(),
            depth_bias: material.depth_bias(),
        },
    })
}
