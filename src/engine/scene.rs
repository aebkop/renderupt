use std::{collections::{HashMap, HashSet}, hash::Hash, iter::Map};

use erupt::{vk::{self}};
use nalgebra::Isometry3;

use super::{mesh::Mesh, pipeline::PipelineStruct};


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

}


