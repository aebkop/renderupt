use std::{collections::{HashMap, HashSet}, hash::Hash, iter::Map};

use erupt::{vk::{self}};
use gpu_alloc_erupt::EruptMemoryDevice;
use nalgebra::Isometry3;

use super::{device::Physical, mesh::Mesh, pipeline::PipelineStruct};


pub struct Material {
    pub pipeline: PipelineStruct
}



pub struct Scene {
    pub objects: Vec<(String, String, nalgebra::Isometry3<f32>)>,
    pub meshes: HashMap<String, Mesh>,
    pub materials: HashMap<String, Material>
} 

impl Scene{
    pub fn new() -> Self {
        Scene {
            objects: Vec::new(),
            meshes: HashMap::new(),
            materials: HashMap::new()
        }
    }


    pub fn add_render_object_with_mesh_material(&mut self, mesh: Mesh, mesh_name: &str, material: Material, material_name: &str, translation_matrix: Isometry3<f32>) {
        self.meshes.insert(mesh_name.to_string(), mesh);
        self.materials.insert(material_name.to_string(), material);
        self.objects.push((mesh_name.to_string(), material_name.to_string(), translation_matrix));
    }

    pub fn add_render_object_with_mesh(&mut self, mesh: Mesh, mesh_name: &str, material_name: &str, translation_matrix: Isometry3<f32>) {
        self.meshes.insert(mesh_name.to_string(), mesh);
        self.objects.push((mesh_name.to_string(), material_name.to_string(), translation_matrix));
    }


    pub fn add_render_object(&mut self, mesh_name: &str, material_name: &str, translation_matrix: Isometry3<f32>) {
        self.objects.push((mesh_name.to_string(), material_name.to_string(), translation_matrix));
    
    }

    pub fn cleanup(&mut self, physical: &mut Physical) {
        unsafe { 
        for (_, mesh) in self.meshes.iter_mut() {
            physical.allocator.dealloc(EruptMemoryDevice::wrap(&physical.device),mesh.vertex_buffer.allocation.take().unwrap());
            physical.device.destroy_buffer(Some(mesh.vertex_buffer.buffer), None);
        }
        for (_, material) in self.materials.iter() {
            physical.device.destroy_pipeline_layout(Some(material.pipeline.pipeline_layout), None);
            for pipeline in &material.pipeline.pipelines {
                physical.device.destroy_pipeline(Some(*pipeline), None);
            }
        }
    }}
}


