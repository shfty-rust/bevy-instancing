use std::{hash::Hash, marker::PhantomData};

use bevy::{
    asset::{AssetServer, Handle},
    ecs::{prelude::World, world::FromWorld},
    pbr::MeshPipelineKey,
    render::{
        mesh::MeshVertexBufferLayout,
        render_resource::{
            BindGroupLayout, RenderPipelineDescriptor, Shader, SpecializedMeshPipeline,
            SpecializedMeshPipelineError,
        },
        renderer::RenderDevice,
    },
};

use crate::prelude::{InstancedMeshPipeline, MaterialInstanced};

pub struct InstancedMaterialPipelineKey<M: MaterialInstanced> {
    pub mesh_key: MeshPipelineKey,
    pub material_key: M::Data,
}

impl<M: MaterialInstanced> Clone for InstancedMaterialPipelineKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            mesh_key: self.mesh_key.clone(),
            material_key: self.material_key.clone(),
        }
    }
}

impl<M: MaterialInstanced> PartialEq for InstancedMaterialPipelineKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.mesh_key == other.mesh_key && self.material_key == other.material_key
    }
}

impl<M: MaterialInstanced> Eq for InstancedMaterialPipelineKey<M> where M::Data: Eq {}

impl<M: MaterialInstanced> Hash for InstancedMaterialPipelineKey<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.mesh_key.hash(state);
        self.material_key.hash(state);
    }
}

pub struct InstancedMaterialPipeline<M: MaterialInstanced> {
    pub instanced_mesh_pipeline: InstancedMeshPipeline,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    marker: PhantomData<M>,
}

impl<M: MaterialInstanced> SpecializedMeshPipeline for InstancedMaterialPipeline<M>
where
    M::Data: Clone + Hash + PartialEq + Eq,
{
    type Key = InstancedMaterialPipelineKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self
            .instanced_mesh_pipeline
            .specialize(key.mesh_key, layout)?;
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        // MeshPipeline::specialize's current implementation guarantees that the returned
        // specialized descriptor has a populated layout
        let descriptor_layout = descriptor.layout.as_mut().unwrap();
        descriptor_layout.insert(1, self.material_layout.clone());

        M::specialize(self, &mut descriptor, key.material_key, layout)?;
        Ok(descriptor)
    }
}

impl<M: MaterialInstanced> FromWorld for InstancedMaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        let material_layout = M::bind_group_layout(render_device);

        InstancedMaterialPipeline {
            instanced_mesh_pipeline: world.resource::<InstancedMeshPipeline>().clone(),
            material_layout,
            vertex_shader: M::vertex_shader(asset_server),
            fragment_shader: M::fragment_shader(asset_server),
            marker: PhantomData,
        }
    }
}
