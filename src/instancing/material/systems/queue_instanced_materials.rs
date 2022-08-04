use std::hash::Hash;

use bevy::{
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
    pbr::MeshPipelineKey,
    prelude::{debug, error, Commands, Entity, Msaa, Query, Res, ResMut, With},
    render::{
        render_phase::{DrawFunctions, RenderPhase},
        render_resource::{PipelineCache, SpecializedMeshPipelines},
        view::{ExtractedView, VisibleEntities},
    },
};

use crate::instancing::material::{
    instanced_material_pipeline::{InstancedMaterialPipeline, InstancedMaterialPipelineKey},
    material_instanced::MaterialInstanced,
    plugin::{DrawInstanced, GpuAlphaMode, InstanceMeta},
};

use super::prepare_material_batches::MaterialBatches;

#[allow(clippy::too_many_arguments)]
pub fn system<M: MaterialInstanced>(
    material_batches: Res<MaterialBatches<M>>,
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    instanced_material_pipeline: Res<InstancedMaterialPipeline<M>>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<InstancedMaterialPipeline<M>>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    query_view: Query<(Entity, &InstanceMeta<M>), (With<ExtractedView>, With<VisibleEntities>)>,
    mut query_opaque_3d: Query<&mut RenderPhase<Opaque3d>>,
    mut query_alpha_mask_3d: Query<&mut RenderPhase<AlphaMask3d>>,
    mut query_transparent_3d: Query<&mut RenderPhase<Transparent3d>>,
    mut commands: Commands,
) where
    M::Data: Clone + Hash + PartialEq + Eq,
{
    debug!("{}", std::any::type_name::<M>());

    for (view_entity, instance_meta) in query_view.iter() {
        debug!("\tView {view_entity:?}");

        for key in instance_meta.batched_instances.keys() {
            debug!("{key:#?}");

            // Spawn entity
            let material = material_batches
                .get(&key.material_key)
                .unwrap()
                .material
                .clone_weak();

            let batch_entity = commands.spawn().insert(material).insert(key.clone()).id();

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

            let material_batch = material_batches.get(&key.material_key).unwrap();

            let pipeline = pipelines.specialize(
                &mut pipeline_cache,
                &instanced_material_pipeline,
                InstancedMaterialPipelineKey {
                    mesh_key,
                    material_key: material_batch.pipeline_key.clone(),
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
                    debug!("\t\tQueuing opaque instanced draw {batch_entity:?}");
                    let mut opaque_phase = query_opaque_3d.get_mut(view_entity).unwrap();
                    opaque_phase.add(Opaque3d {
                        entity: batch_entity,
                        draw_function,
                        pipeline,
                        distance,
                    });
                }
                GpuAlphaMode::Mask => {
                    debug!("\t\tQueuing masked instanced draw {batch_entity:?}");
                    let mut alpha_mask_phase = query_alpha_mask_3d.get_mut(view_entity).unwrap();
                    alpha_mask_phase.add(AlphaMask3d {
                        entity: batch_entity,
                        draw_function,
                        pipeline,
                        distance,
                    });
                }
                GpuAlphaMode::Blend => {
                    debug!("\t\tQueuing transparent instanced draw {batch_entity:?}");
                    let mut transparent_phase = query_transparent_3d.get_mut(view_entity).unwrap();
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
