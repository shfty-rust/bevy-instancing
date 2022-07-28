use bevy::{
    app::{App, Plugin},
    asset::AddAsset,
    core::FloatOrd,
    core_pipeline::{AlphaMask3d, Opaque3d, Transparent3d},
    ecs::{
        component::TableStorage,
        system::{
            lifetimeless::{Read, SQuery, SRes},
            Query, Res, ResMut, SystemParamItem,
        },
    },
    pbr::{AlphaMode, MeshPipelineKey, SetMeshViewBindGroup},
    prelude::{
        debug, default, error, Assets, Commands, Deref, DerefMut, Entity, Handle, Mesh,
        ParallelSystemDescriptorCoercion, With,
    },
    render::{
        mesh::{Indices, MeshVertexBufferLayout, PrimitiveTopology},
        render_asset::{PrepareAssetLabel, RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{IndexFormat, PipelineCache, SpecializedMeshPipelines},
        view::{ExtractedView, Msaa, VisibleEntities},
        RenderApp, RenderStage,
    },
    utils::{HashMap, HashSet, Hashed},
};
use bevy::{
    prelude::Component,
    render::{
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferInitDescriptor,
            BufferUsages,
        },
        renderer::RenderDevice,
    },
};

use crate::prelude::{
    extract_mesh_instances, DrawIndexedIndirect, DrawIndirect, Instance, InstanceBlock,
    InstanceBlockBuffer, InstanceBlockRange, InstancedMaterialPipeline,
    InstancedMaterialPipelineKey, SetInstancedMaterialBindGroup, SetInstancedMeshBindGroup,
    SpecializedInstancedMaterial,
};

use std::collections::{BTreeMap, BTreeSet};

use std::marker::PhantomData;

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
    M::Key: PartialOrd + Ord + std::fmt::Debug,
{
    fn build(&self, app: &mut App) {
        app.add_asset::<M>()
            .add_plugin(ExtractComponentPlugin::<Handle<M>>::default())
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
                .add_system_to_stage(RenderStage::Extract, extract_instanced_meshes::<M>)
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_instanced_view_meta::<M>
                        .before(prepare_view_instances::<M>)
                        .before(prepare_view_instance_blocks::<M>),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_view_instances::<M>.before(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_view_instance_blocks::<M>.before(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_mesh_batches::<M>.after(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_material_batches::<M>.after(PrepareAssetLabel::AssetPrepare),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_instance_batches::<M>
                        .after(prepare_mesh_batches::<M>)
                        .after(prepare_material_batches::<M>),
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_instanced_materials::<M>.after(prepare_instance_batches::<M>),
                )
                .add_system_to_stage(RenderStage::Queue, queue_instanced_materials::<M>);
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
    pub key: Hashed<InstancedMeshKey>,
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
    pub instanced_meshes: HashMap<Handle<Mesh>, GpuInstancedMesh>,
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

fn extract_instanced_meshes<M: SpecializedInstancedMaterial>(
    meshes: Res<Assets<Mesh>>,
    query_mesh: Query<&Handle<Mesh>, With<Handle<M>>>,
    mut commands: Commands,
) {
    let mut instanced_meshes = HashMap::new();

    for mesh_handle in query_mesh.iter().collect::<HashSet<_>>() {
        let mesh = meshes.get(mesh_handle).unwrap();
        let vertex_buffer_data = mesh.get_vertex_buffer_data();
        let vertex_count = mesh.count_vertices();

        let index_buffer_data = mesh.indices().map_or(
            GpuIndexBufferData::NonIndexed {
                vertex_count: vertex_count as u32,
            },
            |indices| -> GpuIndexBufferData {
                GpuIndexBufferData::Indexed {
                    indices: indices.clone(),
                    index_count: mesh.indices().unwrap().len() as u32,
                    index_format: mesh.indices().unwrap().into(),
                }
            },
        );

        let mesh_vertex_buffer_layout = mesh.get_mesh_vertex_buffer_layout();

        let primitive_topology = mesh.primitive_topology();

        let key = InstancedMeshKey {
            primitive_topology,
            layout: mesh_vertex_buffer_layout.clone(),
            index_format: match index_buffer_data {
                GpuIndexBufferData::Indexed { index_format, .. } => Some(index_format),
                GpuIndexBufferData::NonIndexed { .. } => None,
            },
        };

        let key = Hashed::new(key);

        instanced_meshes.insert(
            mesh_handle.clone_weak(),
            GpuInstancedMesh {
                key,
                vertex_buffer_data,
                vertex_count,
                index_buffer_data,
                primitive_topology: mesh.primitive_topology(),
                layout: mesh_vertex_buffer_layout,
            },
        );
    }

    commands.insert_resource(GpuInstancedMeshes::<M> {
        instanced_meshes,
        ..default()
    })
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DrawIndirectVariant {
    NonIndexed(DrawIndirect),
    Indexed(DrawIndexedIndirect),
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
pub struct InstancedMaterialKey<M: SpecializedInstancedMaterial> {
    alpha_mode: GpuAlphaMode,
    key: M::Key,
}

impl<M: SpecializedInstancedMaterial> Clone for InstancedMaterialKey<M> {
    fn clone(&self) -> Self {
        Self {
            alpha_mode: self.alpha_mode.clone(),
            key: self.key.clone(),
        }
    }
}

impl<M: SpecializedInstancedMaterial> PartialEq for InstancedMaterialKey<M> {
    fn eq(&self, other: &Self) -> bool {
        self.alpha_mode == other.alpha_mode && self.key == other.key
    }
}

impl<M: SpecializedInstancedMaterial> Eq for InstancedMaterialKey<M> {}

impl<M: SpecializedInstancedMaterial> PartialOrd for InstancedMaterialKey<M>
where
    M::Key: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.alpha_mode.partial_cmp(&other.alpha_mode) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.key.partial_cmp(&other.key)
    }
}

impl<M: SpecializedInstancedMaterial> Ord for InstancedMaterialKey<M>
where
    M::Key: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.alpha_mode.cmp(&other.alpha_mode) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.key.cmp(&other.key)
    }
}

impl<M: SpecializedInstancedMaterial> std::hash::Hash for InstancedMaterialKey<M> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.alpha_mode.hash(state);
        self.key.hash(state);
    }
}

impl<M: SpecializedInstancedMaterial> std::fmt::Debug for InstancedMaterialKey<M>
where
    M::Key: std::fmt::Debug,
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
    mesh_key: Hashed<InstancedMeshKey>,
    material_key: InstancedMaterialKey<M>,
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
    M::Key: PartialOrd,
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
    M::Key: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.mesh_key.cmp(&other.mesh_key) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.material_key.cmp(&other.material_key)
    }
}

impl<M: SpecializedInstancedMaterial> std::hash::Hash for InstanceBatchKey<M> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.mesh_key.hash();
        self.material_key.hash(state);
    }
}

impl<M: SpecializedInstancedMaterial> std::fmt::Debug for InstanceBatchKey<M>
where
    M::Key: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceKey")
            .field("mesh_key", &self.mesh_key)
            .field("material_key", &self.material_key)
            .finish()
    }
}

#[derive(Clone)]
pub struct MeshBatch {
    meshes: BTreeSet<Handle<Mesh>>,
    vertex_data: Vec<u8>,
    index_data: Option<GpuIndexBufferData>,
    indirect_data: Vec<DrawIndirectVariant>,
}

pub struct InstanceBatch<M: SpecializedInstancedMaterial> {
    pub instances: BTreeSet<Entity>,
    pub instance_block_ranges: BTreeMap<Entity, InstanceBlockRange>,
    pub instance_buffer_data:
        Vec<<<M as SpecializedInstancedMaterial>::Instance as Instance>::PreparedInstance>,
}

#[derive(Deref, DerefMut)]
pub struct InstanceViewMeta<M: SpecializedInstancedMaterial> {
    pub view_meta: HashMap<Entity, InstanceMeta<M>>,
}

impl<M: SpecializedInstancedMaterial> Default for InstanceViewMeta<M> {
    fn default() -> Self {
        Self {
            view_meta: default(),
        }
    }
}

/// Resource containing instance batches
pub struct InstanceMeta<M: SpecializedInstancedMaterial> {
    pub instances: Vec<Entity>,
    pub instance_blocks: Vec<Entity>,
    pub mesh_batches: HashMap<Hashed<InstancedMeshKey>, MeshBatch>,
    pub material_batches: HashMap<InstancedMaterialKey<M>, Handle<M>>,
    pub instance_batches: HashMap<InstanceBatchKey<M>, InstanceBatch<M>>,
    pub batched_instances: HashMap<InstanceBatchKey<M>, BatchedInstances>,
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

/// The data necessary to render one set of mutually compatible instances
#[derive(Debug, Clone, Component)]
pub struct BatchedInstances {
    pub view_entity: Entity,
    pub batch_entity: Entity,
    pub vertex_buffer: Buffer,
    pub index_data: Option<(Buffer, IndexFormat)>,
    pub indirect_buffer: Buffer,
    pub instance_bind_group: BindGroup,
    pub indirect_count: usize,
    pub indirect_indices: Vec<usize>,
}

pub fn prepare_instanced_view_meta<M: SpecializedInstancedMaterial>(
    query_views: Query<Entity, With<VisibleEntities>>,
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
) {
    instance_view_meta.clear();
    for view_entity in query_views.iter() {
        instance_view_meta.insert(view_entity, default());
    }
}

pub fn prepare_view_instances<M: SpecializedInstancedMaterial>(
    query_views: Query<(Entity, &VisibleEntities)>,
    query_instance: Query<
        (Entity,),
        (
            With<Handle<M>>,
            With<<M::Instance as Instance>::ExtractedInstance>,
        ),
    >,
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
) {
    for (view_entity, visible_entities) in query_views.iter() {
        instance_view_meta.get_mut(&view_entity).unwrap().instances = visible_entities
            .entities
            .iter()
            .copied()
            .filter(|entity| query_instance.get(*entity).is_ok())
            .collect::<Vec<_>>();
    }
}

pub fn prepare_view_instance_blocks<M: SpecializedInstancedMaterial>(
    query_views: Query<(Entity, &VisibleEntities)>,
    query_instance_block: Query<Entity, (With<Handle<M>>, With<InstanceBlock>)>,
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
) {
    for (view_entity, visible_entities) in query_views.iter() {
        instance_view_meta
            .get_mut(&view_entity)
            .unwrap()
            .instance_blocks = visible_entities
            .entities
            .iter()
            .copied()
            .filter(|entity| query_instance_block.get(*entity).is_ok())
            .collect::<Vec<_>>();
    }
}

pub fn prepare_mesh_batches<M: SpecializedInstancedMaterial>(
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
    render_meshes: Res<GpuInstancedMeshes<M>>,
    query_views: Query<Entity, With<ExtractedView>>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_block: Query<(Entity, &Handle<M>, &InstanceBlock)>,
) where
    M::Key: PartialOrd + Ord + std::fmt::Debug,
{
    let render_meshes = &render_meshes.instanced_meshes;

    for view_entity in query_views.iter() {
        let instance_meta = instance_view_meta.get_mut(&view_entity).unwrap();

        // Collect set of visible meshes
        let meshes = instance_meta
            .instances
            .iter()
            .flat_map(|entity| query_instance.get(*entity))
            .map(|(_, _, instance)| <M::Instance as Instance>::mesh(instance).clone_weak())
            .chain(
                instance_meta
                    .instance_blocks
                    .iter()
                    .flat_map(|entity| query_instance_block.get(*entity))
                    .map(|(_, _, instance_block)| instance_block.mesh.clone_weak()),
            )
            .collect::<HashSet<_>>();

        // Sort meshes into batches by their InstancedMeshKey
        let mut keyed_meshes = HashMap::<Hashed<InstancedMeshKey>, BTreeSet<Handle<Mesh>>>::new();
        for mesh_handle in meshes.into_iter() {
            let mesh = render_meshes.get(&mesh_handle).unwrap();
            keyed_meshes
                .entry(mesh.key.clone())
                .or_default()
                .insert(mesh_handle);
        }

        // Generate vertex, index, and indirect data for each batch
        instance_meta.mesh_batches = keyed_meshes
            .into_iter()
            .map(|(key, meshes)| {
                let vertex_data = meshes
                    .iter()
                    .flat_map(|mesh| {
                        let mesh = render_meshes.get(mesh).unwrap();
                        mesh.vertex_buffer_data.iter().copied()
                    })
                    .collect::<Vec<_>>();

                let mut base_index = 0;
                let index_data = meshes.iter().fold(None, |acc, mesh| {
                    let mesh = render_meshes.get(mesh).unwrap();

                    let out = match &mesh.index_buffer_data {
                        GpuIndexBufferData::Indexed {
                            indices,
                            index_count,
                            index_format,
                        } => Some(match acc {
                            Some(GpuIndexBufferData::Indexed {
                                indices: acc_indices,
                                index_count: acc_index_count,
                                ..
                            }) => GpuIndexBufferData::Indexed {
                                indices: match (acc_indices, indices) {
                                    (Indices::U16(lhs), Indices::U16(rhs)) => Indices::U16(
                                        lhs.iter()
                                            .copied()
                                            .chain(rhs.iter().map(|idx| base_index as u16 + *idx))
                                            .collect(),
                                    ),
                                    (Indices::U32(lhs), Indices::U32(rhs)) => Indices::U32(
                                        lhs.iter()
                                            .copied()
                                            .chain(rhs.iter().map(|idx| base_index as u32 + *idx))
                                            .collect(),
                                    ),
                                    _ => panic!("Mismatched index format"),
                                },

                                index_count: index_count + acc_index_count,
                                index_format: *index_format,
                            },
                            None => GpuIndexBufferData::Indexed {
                                indices: indices.clone(),
                                index_count: *index_count,
                                index_format: *index_format,
                            },
                            _ => panic!("Mismatched GpuIndexBufferData"),
                        }),
                        GpuIndexBufferData::NonIndexed { vertex_count } => Some(match acc {
                            Some(GpuIndexBufferData::NonIndexed {
                                vertex_count: acc_vertex_count,
                            }) => GpuIndexBufferData::NonIndexed {
                                vertex_count: vertex_count + acc_vertex_count,
                            },
                            None => GpuIndexBufferData::NonIndexed {
                                vertex_count: *vertex_count,
                            },
                            _ => panic!("Mismatched GpuIndexBufferData"),
                        }),
                    };

                    base_index += mesh.vertex_count;

                    out
                });

                let mut base_index = 0u32;
                let indirect_data = meshes
                    .iter()
                    .map(
                        |mesh| match &render_meshes.get(mesh).unwrap().index_buffer_data {
                            GpuIndexBufferData::Indexed { index_count, .. } => {
                                base_index += index_count;

                                DrawIndirectVariant::Indexed(DrawIndexedIndirect {
                                    index_count: *index_count,
                                    ..default()
                                })
                            }
                            GpuIndexBufferData::NonIndexed { vertex_count } => {
                                base_index += vertex_count;

                                DrawIndirectVariant::NonIndexed(DrawIndirect {
                                    vertex_count: *vertex_count,
                                    ..default()
                                })
                            }
                        },
                    )
                    .collect::<Vec<_>>();

                (
                    key.clone(),
                    MeshBatch {
                        meshes,
                        vertex_data,
                        index_data,
                        indirect_data,
                    },
                )
            })
            .collect();
    }
}

pub fn prepare_instance_batches<M: SpecializedInstancedMaterial>(
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
    render_meshes: Res<GpuInstancedMeshes<M>>,
    render_materials: Res<RenderAssets<M>>,
    mut query_views: Query<(Entity, &ExtractedView)>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_block: Query<(Entity, &Handle<M>, &InstanceBlock)>,
) where
    M::Key: PartialOrd + Ord + std::fmt::Debug,
{
    debug!("prepare_instance_batches<{}>", std::any::type_name::<M>());

    let render_meshes = &render_meshes.instanced_meshes;

    for (view_entity, view) in query_views.iter_mut() {
        let instance_meta = instance_view_meta.get_mut(&view_entity).unwrap();

        // Fetch view matrix for sorting
        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_matrix.row(2);

        let span = bevy::prelude::info_span!("Batch instances by key");
        let mut keyed_instances = span.in_scope(|| {
            // Batch instances by key
            let mut keyed_instances = BTreeMap::<
                InstanceBatchKey<M>,
                Vec<(
                    Entity,
                    Handle<Mesh>,
                    &Handle<M>,
                    FloatOrd,
                    &<M::Instance as Instance>::ExtractedInstance,
                )>,
            >::new();

            for (entity, material_handle, instance) in instance_meta
                .instances
                .iter()
                .flat_map(|entity| query_instance.get(*entity))
            {
                let mesh = render_meshes
                    .get(<M::Instance as Instance>::mesh(instance))
                    .unwrap();
                let mesh_key = mesh.key.clone();

                let material = render_materials.get(material_handle).unwrap();
                let alpha_mode = GpuAlphaMode::from(M::alpha_mode(material));
                let material_key = InstancedMaterialKey {
                    alpha_mode,
                    key: M::key(material),
                };

                let mesh_z =
                    inverse_view_row_2.dot(<M::Instance as Instance>::transform(instance).col(3));

                let dist = mesh_z
                    * if alpha_mode == GpuAlphaMode::Blend {
                        // Back-to-front ordering
                        1.0
                    } else {
                        // Front-to-back ordering
                        -1.0
                    };

                let key = InstanceBatchKey {
                    mesh_key,
                    material_key,
                };

                keyed_instances.entry(key).or_default().push((
                    entity,
                    <M::Instance as Instance>::mesh(instance).clone_weak(),
                    material_handle,
                    FloatOrd(dist),
                    instance,
                ));
            }

            keyed_instances
        });

        let span = bevy::prelude::info_span!("Sort instances by mesh and distance");
        span.in_scope(|| {
            // Sort instances by mesh and distance
            for instances in keyed_instances.values_mut() {
                instances.sort_by(
                    |(_, lhs_mesh, _, lhs_dist, _), (_, rhs_mesh, _, rhs_dist, _)| {
                        (lhs_mesh, lhs_dist).cmp(&(rhs_mesh, rhs_dist))
                    },
                );
            }
        });

        let span = bevy::prelude::info_span!("Batch instance blocks by key");
        let keyed_instance_blocks = span.in_scope(|| {
            // Batch instance blocks by key

            let mut keyed_instance_blocks =
                BTreeMap::<InstanceBatchKey<M>, Vec<(Entity, &Handle<M>, &InstanceBlock)>>::new();

            for (entity, material_handle, instance_block) in instance_meta
                .instance_blocks
                .iter()
                .flat_map(|entity| query_instance_block.get(*entity))
            {
                let mesh = render_meshes.get(&instance_block.mesh).unwrap();
                let mesh_key = mesh.key.clone();

                let material = render_materials.get(material_handle).unwrap();
                let alpha_mode = GpuAlphaMode::from(M::alpha_mode(material));
                let material_key = InstancedMaterialKey {
                    alpha_mode,
                    key: M::key(material),
                };

                let key = InstanceBatchKey {
                    mesh_key,
                    material_key,
                };

                keyed_instance_blocks.entry(key).or_default().push((
                    entity,
                    material_handle,
                    instance_block,
                ));
            }

            keyed_instance_blocks
        });

        // Create an instance buffer vec for each key
        let mut keyed_instance_buffer_data =
            BTreeMap::<InstanceBatchKey<M>, Vec<<M::Instance as Instance>::PreparedInstance>>::new(
            );

        let span = bevy::prelude::info_span!("Populate instances");
        span.in_scope(|| {
            // Populate instances
            for (key, instances) in keyed_instances.iter() {
                // Collect instance data
                let instance_buffer_data = instances.iter().map(|(_, _, _, _, instance)| {
                    let MeshBatch { meshes, .. } =
                        instance_meta.mesh_batches.get(&key.mesh_key).unwrap();

                    <M::Instance as Instance>::prepare_instance(
                        instance,
                        meshes
                            .iter()
                            .position(|mesh| mesh == <M::Instance as Instance>::mesh(instance))
                            .unwrap() as u32,
                    )
                });

                keyed_instance_buffer_data
                    .entry(key.clone())
                    .or_default()
                    .extend(instance_buffer_data);
            }
        });

        let span = bevy::prelude::info_span!("Create instance block ranges");
        let mut keyed_instance_block_ranges = span.in_scope(|| {
            // Create instance block ranges
            keyed_instance_blocks
                .iter()
                .map(|(key, instance_blocks)| {
                    let instance_buffer_data_len = keyed_instance_buffer_data
                        .get(&key)
                        .map(Vec::len)
                        .unwrap_or_default();

                    // Collect CPU instance block data
                    let mut offset = instance_buffer_data_len;
                    let mut instance_block_ranges = BTreeMap::<Entity, InstanceBlockRange>::new();
                    for (entity, _, instance_block) in instance_blocks {
                        // Generate instance block range
                        instance_block_ranges.insert(
                            *entity,
                            InstanceBlockRange {
                                offset: offset as u64,
                                instance_count: instance_block.instance_count as u64,
                            },
                        );

                        offset += instance_block.instance_count;
                    }

                    debug!("Instance block ranges: {instance_block_ranges:?}");

                    (key.clone(), instance_block_ranges)
                })
                .collect::<BTreeMap<_, _>>()
        });

        let span = bevy::prelude::info_span!("Populate instance blocks");
        span.in_scope(|| {
            // Populate instance blocks
            for (key, instance_blocks) in keyed_instance_blocks.iter() {
                // Collect instance data
                let instance_count: usize = instance_blocks
                    .iter()
                    .map(|(_, _, instance_block)| instance_block.instance_count)
                    .sum();

                let entry = keyed_instance_buffer_data.entry(key.clone()).or_default();
                entry.resize(entry.len() + instance_count, default());
            }
        });

        let span = bevy::prelude::info_span!("Write instance batches");
        span.in_scope(|| {
            // Write instance batches to meta
            instance_meta
                .instance_batches
                .extend(keyed_instance_buffer_data.into_iter().map(
                    |(key, instance_buffer_data)| {
                        let instances = keyed_instances
                            .remove(&key)
                            .map(|instances| {
                                instances
                                    .into_iter()
                                    .map(|(instance, _, _, _, _)| instance)
                                    .collect::<BTreeSet<_>>()
                            })
                            .unwrap_or_default();

                        let instance_block_ranges =
                            keyed_instance_block_ranges.remove(&key).unwrap_or_default();

                        (
                            key.clone(),
                            InstanceBatch::<M> {
                                instances,
                                instance_block_ranges,
                                instance_buffer_data,
                            },
                        )
                    },
                ));
        });
    }
}

pub fn prepare_material_batches<M: SpecializedInstancedMaterial>(
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
    render_materials: Res<RenderAssets<M>>,
    mut query_views: Query<Entity, With<ExtractedView>>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_block: Query<(Entity, &Handle<M>, &InstanceBlock)>,
) {
    for view_entity in query_views.iter_mut() {
        let instance_meta = instance_view_meta.get_mut(&view_entity).unwrap();

        // Collect set of visible materials
        let materials = instance_meta
            .instances
            .iter()
            .flat_map(|entity| query_instance.get(*entity))
            .map(|(_, material, _)| material.clone_weak())
            .chain(
                instance_meta
                    .instance_blocks
                    .iter()
                    .flat_map(|entity| query_instance_block.get(*entity))
                    .map(|(_, material, _)| material.clone_weak()),
            )
            .collect::<BTreeSet<_>>();

        // Batch materials by key
        instance_meta.material_batches = materials
            .into_iter()
            .map(|material_handle| {
                let material = render_materials.get(&material_handle).unwrap();
                (
                    InstancedMaterialKey {
                        alpha_mode: M::alpha_mode(material).into(),
                        key: M::key(material),
                    },
                    material_handle,
                )
            })
            .collect();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_instanced_materials<M: SpecializedInstancedMaterial>(
    instanced_material_pipeline: Res<InstancedMaterialPipeline<M>>,
    render_meshes: Res<GpuInstancedMeshes<M>>,
    render_materials: Res<RenderAssets<M>>,
    mut instance_view_meta: ResMut<InstanceViewMeta<M>>,
    render_device: Res<RenderDevice>,
    query_instance: Query<(
        Entity,
        &Handle<M>,
        &<M::Instance as Instance>::ExtractedInstance,
    )>,
    query_instance_block: Query<(Entity, &Handle<M>, &InstanceBlock)>,
    mut query_views: Query<Entity, With<ExtractedView>>,
    mut commands: Commands,
) where
    M::Key: PartialOrd + Ord + std::fmt::Debug,
{
    debug!(
        "prepare_instanced_materials<{}>",
        std::any::type_name::<M>()
    );

    let render_meshes = &render_meshes.instanced_meshes;

    for view_entity in query_views.iter_mut() {
        let instance_meta = instance_view_meta.get_mut(&view_entity).unwrap();

        // Collect set of visible materials
        let materials = instance_meta
            .instances
            .iter()
            .flat_map(|entity| query_instance.get(*entity))
            .map(|(_, material, _)| material.clone_weak())
            .chain(
                instance_meta
                    .instance_blocks
                    .iter()
                    .flat_map(|entity| query_instance_block.get(*entity))
                    .map(|(_, material, _)| material.clone_weak()),
            )
            .collect::<BTreeSet<_>>();

        // Batch materials by key
        instance_meta.material_batches = materials
            .into_iter()
            .map(|material_handle| {
                let material = render_materials.get(&material_handle).unwrap();
                (
                    InstancedMaterialKey {
                        alpha_mode: M::alpha_mode(material).into(),
                        key: M::key(material),
                    },
                    material_handle,
                )
            })
            .collect();

        // Process batches
        let mut batched_instances = HashMap::<InstanceBatchKey<M>, BatchedInstances>::new();
        for (key, instance_batch) in &instance_meta.instance_batches {
            // Fetch data
            let MeshBatch {
                meshes,
                vertex_data,
                index_data,
                indirect_data,
            } = instance_meta.mesh_batches.get(&key.mesh_key).unwrap();

            // Calculate mesh instance counts for this batch
            let mut mesh_instance_counts = BTreeMap::<&Handle<Mesh>, usize>::new();

            for mesh in meshes {
                mesh_instance_counts.insert(mesh, 0);
            }

            let instance_meshes = instance_batch
                .instances
                .iter()
                .flat_map(|entity| query_instance.get(*entity))
                .map(|(_, _, instance)| <M::Instance as Instance>::mesh(instance));

            for mesh in instance_meshes {
                *mesh_instance_counts.get_mut(mesh).unwrap() += 1;
            }

            for instance_block in instance_batch
                .instance_block_ranges
                .iter()
                .flat_map(|(entity, _)| query_instance_block.get(*entity))
                .map(|(_, _, instance_block)| instance_block)
            {
                *mesh_instance_counts.get_mut(&instance_block.mesh).unwrap() +=
                    instance_block.instance_count;
            }

            debug!("Mesh instance counts: {mesh_instance_counts:?}");

            // Calculate instance offsets for this batch
            let (mesh_instance_offsets, _) = mesh_instance_counts.iter().fold(
                (BTreeMap::<&Handle<Mesh>, usize>::new(), 0),
                |(mut offsets, mut offset), (mesh, count)| {
                    offsets.insert(mesh, offset);
                    offset += count;
                    (offsets, offset)
                },
            );

            // Calculate vertex offsets for this batch's mesh
            let (mesh_vertex_offsets, _) = mesh_instance_counts.iter().fold(
                (BTreeMap::<&Handle<Mesh>, usize>::new(), 0),
                |(mut offsets, mut offset), (mesh, _)| {
                    offsets.insert(mesh, offset);

                    let gpu_mesh = render_meshes.get(mesh).unwrap();

                    offset += match gpu_mesh.index_buffer_data {
                        GpuIndexBufferData::Indexed { index_count, .. } => index_count,
                        GpuIndexBufferData::NonIndexed { vertex_count } => vertex_count,
                    } as usize;

                    (offsets, offset)
                },
            );

            // Upload GPU data and create bind groups
            let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("instanced vertex buffer"),
                contents: vertex_data,
                usage: BufferUsages::VERTEX,
            });

            let index_buffer = match index_data {
                Some(GpuIndexBufferData::Indexed { indices, .. }) => Some({
                    render_device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("instanced index buffer"),
                        contents: match indices {
                            Indices::U16(indices) => bytemuck::cast_slice(indices),
                            Indices::U32(indices) => bytemuck::cast_slice(indices),
                        },
                        usage: BufferUsages::INDEX,
                    })
                }),
                _ => None,
            };

            let indirect_buffer = match key.mesh_key.index_format {
                Some(_) => {
                    let indirect_data = indirect_data
                        .into_iter()
                        .zip(
                            mesh_instance_counts.values().zip(
                                mesh_vertex_offsets
                                    .values()
                                    .zip(mesh_instance_offsets.values()),
                            ),
                        )
                        .map(
                            |(variant, (instance_count, (index_offset, instance_offset)))| {
                                let indirect =
                                    if let DrawIndirectVariant::Indexed(indirect) = variant {
                                        indirect
                                    } else {
                                        panic!("Mismatched DrawIndirectVariant");
                                    };

                                let indirect = DrawIndexedIndirect {
                                    instance_count: *instance_count as u32,
                                    first_index: *index_offset as u32,
                                    first_instance: *instance_offset as u32,
                                    ..*indirect
                                };

                                indirect
                            },
                        )
                        .collect::<Vec<_>>();

                    render_device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("indirect buffer"),
                        contents: bytemuck::cast_slice(&indirect_data),
                        usage: BufferUsages::INDIRECT,
                    })
                }
                None => {
                    let indirect_data = indirect_data
                        .into_iter()
                        .zip(
                            mesh_instance_counts.values().zip(
                                mesh_vertex_offsets
                                    .values()
                                    .zip(mesh_instance_offsets.values()),
                            ),
                        )
                        .map(
                            |(variant, (instance_count, (vertex_offset, instance_offset)))| {
                                let indirect =
                                    if let DrawIndirectVariant::NonIndexed(indirect) = variant {
                                        *indirect
                                    } else {
                                        panic!("Mismatched DrawIndirectVariant");
                                    };

                                DrawIndirect {
                                    instance_count: *instance_count as u32,
                                    first_vertex: *vertex_offset as u32,
                                    first_instance: *instance_offset as u32,
                                    ..indirect
                                }
                            },
                        )
                        .collect::<Vec<_>>();

                    render_device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("indirect buffer"),
                        contents: bytemuck::cast_slice(&indirect_data),
                        usage: BufferUsages::INDIRECT,
                    })
                }
            };

            let instance_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("instance buffer"),
                contents: bytemuck::cast_slice(&instance_batch.instance_buffer_data),
                usage: BufferUsages::STORAGE,
            });

            let indirect_indices = mesh_instance_counts
                .iter()
                .enumerate()
                .flat_map(|(i, (_, count))| if *count > 0 { Some(i) } else { None })
                .collect::<Vec<_>>();

            debug!("Indirect indices: {indirect_indices:#?}");

            let instance_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("instance bind group"),
                layout: &instanced_material_pipeline
                    .instanced_mesh_pipeline
                    .bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: instance_buffer.as_entire_binding(),
                }],
            });

            // Insert instance block data
            for (entity, block_range) in instance_meta
                .instance_batches
                .get(key)
                .unwrap()
                .instance_block_ranges
                .iter()
            {
                commands
                    .entity(*entity)
                    .insert(*block_range)
                    .insert(InstanceBlockBuffer {
                        buffer: instance_buffer.clone(),
                    });
            }

            // Spawn entity
            let material = instance_meta
                .material_batches
                .get(&key.material_key)
                .unwrap();

            let batch_entity = commands
                .spawn()
                .insert(material.clone_weak())
                .insert(key.clone())
                .id();

            // Insert meta
            let indirect_count = indirect_data.len();
            batched_instances.insert(
                key.clone(),
                BatchedInstances {
                    view_entity,
                    batch_entity,
                    vertex_buffer,
                    index_data: index_buffer
                        .map(|index_buffer| (index_buffer, key.mesh_key.index_format.unwrap())),
                    indirect_buffer,
                    instance_bind_group,
                    indirect_count,
                    indirect_indices,
                },
            );
        }

        instance_meta.batched_instances.extend(batched_instances);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_instanced_materials<M: SpecializedInstancedMaterial>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    instanced_material_pipeline: Res<InstancedMaterialPipeline<M>>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<InstancedMaterialPipeline<M>>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    instance_view_meta: ResMut<InstanceViewMeta<M>>,
    mut query_opaque_3d: Query<&mut RenderPhase<Opaque3d>>,
    mut query_alpha_mask_3d: Query<&mut RenderPhase<AlphaMask3d>>,
    mut query_transparent_3d: Query<&mut RenderPhase<Transparent3d>>,
) where
    M::Key: PartialOrd + Ord + std::fmt::Debug,
{
    debug!("queue_instanced_materials<{}>", std::any::type_name::<M>());

    for (view_entity, instance_meta) in instance_view_meta.iter() {
        for (key, batched_instances) in &instance_meta.batched_instances {
            let batch_entity = batched_instances.batch_entity;

            // Queue draw function
            let draw_function = match key.material_key.alpha_mode {
                GpuAlphaMode::Opaque => opaque_draw_functions.read().get_id::<DrawInstanced<M>>(),
                GpuAlphaMode::Mask => alpha_mask_draw_functions
                    .read()
                    .get_id::<DrawInstanced<M>>(),
                GpuAlphaMode::Blend => transparent_draw_functions
                    .read()
                    .get_id::<DrawInstanced<M>>(),
            }
            .unwrap();

            let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

            let mut mesh_key =
                MeshPipelineKey::from_primitive_topology(key.mesh_key.primitive_topology)
                    | msaa_key;

            if let GpuAlphaMode::Blend = key.material_key.alpha_mode {
                mesh_key |= MeshPipelineKey::TRANSPARENT_MAIN_PASS;
            }

            let pipeline = pipelines.specialize(
                &mut pipeline_cache,
                &instanced_material_pipeline,
                InstancedMaterialPipelineKey {
                    mesh_key,
                    material_key: key.material_key.key.clone(),
                },
                &key.mesh_key.layout,
            );

            let pipeline = match pipeline {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            let distance = 0.0;
            match key.material_key.alpha_mode {
                GpuAlphaMode::Opaque => {
                    debug!("Queuing opaque instanced draw {batch_entity:?}");
                    let mut opaque_phase = query_opaque_3d.get_mut(*view_entity).unwrap();
                    opaque_phase.add(Opaque3d {
                        entity: batch_entity,
                        draw_function,
                        pipeline,
                        distance,
                    });
                }
                GpuAlphaMode::Mask => {
                    debug!("Queuing masked instanced draw {batch_entity:?}");
                    let mut alpha_mask_phase = query_alpha_mask_3d.get_mut(*view_entity).unwrap();
                    alpha_mask_phase.add(AlphaMask3d {
                        entity: batch_entity,
                        draw_function,
                        pipeline,
                        distance,
                    });
                }
                GpuAlphaMode::Blend => {
                    debug!("Queuing transparent instanced draw {batch_entity:?}");
                    let mut transparent_phase = query_transparent_3d.get_mut(*view_entity).unwrap();
                    transparent_phase.add(Transparent3d {
                        entity: batch_entity,
                        draw_function,
                        pipeline,
                        distance,
                    });
                }
            }
        }
    }
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
    type Param = (SRes<InstanceViewMeta<M>>, SQuery<Read<InstanceBatchKey<M>>>);
    #[inline]
    fn render<'w>(
        view: Entity,
        item: Entity,
        (instance_meta, query_instance_batch_key): SystemParamItem<'w, '_, Self::Param>,
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

                for i in &batched_instances.indirect_indices {
                    debug!("Drawing indexed indirect {i:?}");
                    pass.draw_indexed_indirect(
                        &batched_instances.indirect_buffer,
                        (i * std::mem::size_of::<DrawIndexedIndirect>()) as u64,
                    );
                }
            }
            None => {
                for i in &batched_instances.indirect_indices {
                    debug!("Drawing indirect {i:?}");
                    pass.draw_indirect(
                        &batched_instances.indirect_buffer,
                        (i * std::mem::size_of::<DrawIndirect>()) as u64,
                    );
                }
            }
        }

        RenderCommandResult::Success
    }
}
