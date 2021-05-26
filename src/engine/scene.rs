use std::{collections::{HashMap, HashSet}, hash::Hash, iter::Map};

use erupt::{vk::{self}};

use super::{mesh::Mesh, pipeline::PipelineStruct};


pub struct Material {
    pipeline: PipelineStruct
}

pub struct RenderObject<'a> {
    mesh: &'a Mesh,
    material: &'a Material
}

pub struct Scene<'a> {
    pub objects: Vec<RenderObject<'a>>,
    pub meshes: HashMap<String, Mesh>,
    pub materials: HashMap<String, Material>
} 

impl<'a> Scene<'a> {
    pub fn new() -> Self {
        Scene {
            objects: Vec::new(),
            meshes: HashMap::new(),
            materials: HashMap::new()
        }
    }


    pub fn add_render_object(&'a mut self, mesh: Mesh, mesh_name: &String, material: Material, material_name: &String) {
        self.meshes.insert(mesh_name.to_string(), mesh);
        self.materials.insert(material_name.to_string(), material);
        let render_object = RenderObject {
            mesh: self.meshes.get(mesh_name).unwrap(),
            material: self.materials.get(material_name).unwrap()
        };
        self.objects.push(render_object);

    }


}

