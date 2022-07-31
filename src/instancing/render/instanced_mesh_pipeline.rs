use bevy::{
    pbr::{MeshPipeline, MeshPipelineKey},
    prelude::{FromWorld, Shader, World},
    render::{
        mesh::MeshVertexBufferLayout,
        render_resource::{
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
            BufferBindingType, RenderPipelineDescriptor, ShaderStages, SpecializedMeshPipeline,
            SpecializedMeshPipelineError,
        },
        renderer::RenderDevice,
    },
};

use crate::prelude::INSTANCED_MESH_SHADER_HANDLE;

/// Pipeline for rendering instanced meshes
#[derive(Clone)]
pub struct InstancedMeshPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub instance_buffer_binding_type: BufferBindingType,
    pub bind_group_layout: BindGroupLayout,
}

impl FromWorld for InstancedMeshPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();

        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();

        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let instance_buffer_binding_type = render_device.get_supported_read_only_binding_type(1);

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("instanced mesh bind group"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: instance_buffer_binding_type,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        InstancedMeshPipeline {
            mesh_pipeline: mesh_pipeline.clone(),
            instance_buffer_binding_type,
            bind_group_layout,
        }
    }
}

impl SpecializedMeshPipeline for InstancedMeshPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        descriptor.label = Some(
            if key.contains(MeshPipelineKey::TRANSPARENT_MAIN_PASS) {
                "transparent_instanced_mesh_pipeline"
            } else {
                "opaque_instanced_mesh_pipeline"
            }
            .into(),
        );

        if !matches!(
            self.instance_buffer_binding_type,
            BufferBindingType::Storage { .. }
        ) {
            descriptor
                .vertex
                .shader_defs
                .push(String::from("NO_STORAGE_BUFFERS_SUPPORT"));
        }

        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.bind_group_layout.clone(),
        ]);

        descriptor.vertex.shader = INSTANCED_MESH_SHADER_HANDLE.typed::<Shader>();

        descriptor.fragment.as_mut().unwrap().shader =
            INSTANCED_MESH_SHADER_HANDLE.typed::<Shader>();

        Ok(descriptor)
    }
}
