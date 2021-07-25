extern crate nalgebra as na;

use std::{
    mem::{size_of, size_of_val},
    u32,
};

use bytemuck_derive::{Pod, Zeroable};
use erupt::vk::{
    self, DeviceMemory, VertexInputAttributeDescriptionBuilder,
    VertexInputBindingDescriptionBuilder,
};
use gpu_alloc::{MemoryBlock, Request, UsageFlags};
use gpu_alloc_erupt::EruptMemoryDevice;
use memoffset::offset_of;
use serde::Serialize;

use super::device::Physical;
#[derive(Debug)]
pub struct AllocatedBuffer {
    pub buffer: vk::Buffer,
    pub allocation: Option<MemoryBlock<DeviceMemory>>,
}

struct AllocatedImage {
    pub image: vk::Image,
    pub allocation: Option<MemoryBlock<DeviceMemory>>,
}

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, Serialize, Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}
#[derive(Debug)]
#[repr(C)]
pub struct Mesh {
    pub verticies: Vec<Vertex>,
    pub vertex_buffer: AllocatedBuffer,
}
#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct push_mesh_constants {
    pub data: na::Vector3<f32>,
    pub render_matrix: na::Matrix4<f32>,
}

pub struct VertexDesc<'a> {
    pub attributes: Vec<VertexInputAttributeDescriptionBuilder<'a>>,
    pub bindings: Vec<VertexInputBindingDescriptionBuilder<'a>>,
}

impl VertexDesc<'_> {
    pub fn new() -> Self {
        let binding_desc = vk::VertexInputBindingDescriptionBuilder::new()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);

        let pos_attr = vk::VertexInputAttributeDescriptionBuilder::new()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(offset_of!(Vertex, pos) as u32);

        let nor_attr = vk::VertexInputAttributeDescriptionBuilder::new()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(offset_of!(Vertex, normal) as u32);

        let col_attr = vk::VertexInputAttributeDescriptionBuilder::new()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(offset_of!(Vertex, color) as u32);

        let bindings = vec![binding_desc];
        let attributes = vec![pos_attr, nor_attr, col_attr];

        return VertexDesc {
            attributes,
            bindings,
        };
    }
}

pub fn load(path: &std::path::Path) -> Vec<Vertex> {
    let mut vertices: Vec<Vertex> = vec![];

    let (models, materials) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            ..Default::default()
        },
    )
    .expect("Failed to OBJ load file");

    let mesh = &models[0].mesh;

    for idx in &mesh.indices {
        let i = *idx as usize;
        let pos = [
            mesh.positions[3 * i],
            mesh.positions[3 * i + 1],
            mesh.positions[3 * i + 2],
        ];
        let normal = if !mesh.normals.is_empty() {
            [
                mesh.normals[3 * i],
                mesh.normals[3 * i + 1],
                mesh.normals[3 * i + 2],
            ]
        } else {
            [0.0, 0.0, 0.0]
        };
        let color = normal.clone();

        vertices.push(Vertex { pos, normal, color })
    }

    println!("{:?}", vertices.len() as f32);

    vertices
}
impl Mesh {
    pub fn new(path: &std::path::Path, physical: &mut Physical) -> Self {
        //     let triangle_data = load(path);
        let mut triangle_data: Vec<Vertex> = vec![];

        let (models, materials) = tobj::load_obj(
            path,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )
        .expect("Failed to OBJ load file");

        let mesh = &models[0].mesh;

        for idx in &mesh.indices {
            let i = *idx as usize;
            let pos = [
                mesh.positions[3 * i],
                mesh.positions[3 * i + 1],
                mesh.positions[3 * i + 2],
            ];
            let normal = if !mesh.normals.is_empty() {
                [
                    mesh.normals[3 * i],
                    mesh.normals[3 * i + 1],
                    mesh.normals[3 * i + 2],
                ]
            } else {
                [0.0, 0.0, 0.0]
            };
            let color = normal.clone();

            triangle_data.push(Vertex { pos, normal, color })
        }

        println!("path {:?}, len {}", path, triangle_data.len());
        let data: &[u8] = bytemuck::cast_slice(&triangle_data);
        let size = size_of_val(data);

        let buffer_info = vk::BufferCreateInfoBuilder::new()
            .size(size as u64)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER);

        let mut block = unsafe {
            physical.allocator.alloc(
                EruptMemoryDevice::wrap(&physical.device),
                Request {
                    size: size as u64,
                    align_mask: 1,
                    usage: UsageFlags::HOST_ACCESS,
                    memory_types: !0,
                },
            )
        }
        .unwrap();

        unsafe {
            block
                .write_bytes(EruptMemoryDevice::wrap(&physical.device), 0, data)
                .unwrap();
        }

        let buffer = unsafe { physical.device.create_buffer(&buffer_info, None, None) }.unwrap();

        unsafe {
            physical
                .device
                .bind_buffer_memory(buffer, *block.memory(), 0)
                .unwrap();
        }

        let allocated_buff = AllocatedBuffer {
            buffer,
            allocation: Some(block),
        };

        Mesh {
            verticies: triangle_data,
            vertex_buffer: (allocated_buff),
        }
    }
}
