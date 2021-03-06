use std::mem::size_of;

use erupt::vk;
use gpu_alloc_erupt::EruptMemoryDevice;

extern crate nalgebra as na;

use super::{buffer::create_buffer, descriptors::Descriptors, device::Physical, mesh::AllocatedBuffer};

use bytemuck_derive::{Pod, Zeroable};

pub struct Frame {
    pub present_semaphore: vk::Semaphore,
    pub render_semaphore: vk::Semaphore,
    pub render_fence: vk::Fence,
    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,
    pub camera_buffer: AllocatedBuffer,
    pub global_descriptor: vk::DescriptorSet,
}
#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct GPUCameraData {
    pub view: na::Matrix4<f32>,
    pub projection: na::Matrix4<f32>,
    pub viewproj: na::Matrix4<f32>,
}

pub struct Frames {
    pub frames: Vec<Frame>,
}

impl Frames {
    pub fn new(frame_count: u32, physical: &mut Physical, descs: &mut Descriptors) -> Self {
        let mut frames: Vec<Frame> = Vec::with_capacity(2);
        for i in 0..frame_count {
            let fence_info =
                vk::FenceCreateInfoBuilder::new().flags(vk::FenceCreateFlags::SIGNALED);
            //we want to create the fence with the Create Signaled flag, so we can wait on it before using it on a GPU command (for the first frame)

            //don't need any info for the semaphore
            let semaphore_create_info = vk::SemaphoreCreateInfoBuilder::new();

            let render_semaphore = unsafe {
                physical
                    .device
                    .create_semaphore(&semaphore_create_info, None, None)
            }
            .unwrap();
            let present_semaphore = unsafe {
                physical
                    .device
                    .create_semaphore(&semaphore_create_info, None, None)
            }
            .unwrap();

            let render_fence =
                unsafe { physical.device.create_fence(&fence_info, None, None) }.unwrap();

            let command_pool_info = vk::CommandPoolCreateInfoBuilder::new()
                .queue_family_index(physical.graphics_queue_family)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

            let command_pool = unsafe {
                physical
                    .device
                    .create_command_pool(&command_pool_info, None, None)
            }
            .unwrap();

            let command_buffer_info = vk::CommandBufferAllocateInfoBuilder::new()
                .command_pool(command_pool)
                .command_buffer_count(1)
                .level(vk::CommandBufferLevel::PRIMARY);

            let command_buffer = unsafe {
                physical
                    .device
                    .allocate_command_buffers(&command_buffer_info)
            }
            .unwrap();

            let buffer = create_buffer(
                physical,
                size_of::<GPUCameraData>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                gpu_alloc::UsageFlags::UPLOAD,
            );

            let global_set_layout = &[descs.global_set_layout];

            let allocate_info = vk::DescriptorSetAllocateInfoBuilder::new()
                .descriptor_pool(descs.descriptor_pool)
                .set_layouts(global_set_layout);

            let global_descriptor = * unsafe {
                physical.device.allocate_descriptor_sets(&allocate_info).unwrap().get(0).unwrap()
            };
            //descriptor buffe for camera r
            let buffer_info = vk::DescriptorBufferInfoBuilder::new()
                .buffer(buffer.buffer)
                .offset(0)
                .range(size_of::<GPUCameraData>() as u64);

            let buffers_info = [buffer_info];

            let write_info = vk::WriteDescriptorSetBuilder::new()
                .dst_binding(0)
                .dst_set(global_descriptor)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffers_info);           

            let copy_desc_info = vk::CopyDescriptorSetBuilder::new()
                .dst_set(global_descriptor)
                .src_set(global_descriptor);
                
            unsafe {
                physical.device.update_descriptor_sets(&[write_info], &[copy_desc_info])
            }
                

            frames.push(Frame {
                present_semaphore,
                render_semaphore,
                render_fence,
                command_pool,
                command_buffer: command_buffer[0],
                camera_buffer: buffer,
                global_descriptor: global_descriptor,
            })
        };
        Frames { frames } 
    }
    pub fn cleanup(&mut self, physical: &mut Physical) {
        for frame in &mut self.frames {
            unsafe {
                physical
                    .device
                    .destroy_semaphore(Some(frame.render_semaphore), None);
                physical
                    .device
                    .destroy_semaphore(Some(frame.present_semaphore), None);
                physical
                    .device
                    .destroy_fence(Some(frame.render_fence), None);
                physical
                    .device
                    .destroy_command_pool(Some(frame.command_pool), None);
                physical
                    .device
                    .destroy_buffer(Some(frame.camera_buffer.buffer), None);

                    
                
                physical.allocator.dealloc(
                    EruptMemoryDevice::wrap(&physical.device),
                    frame.camera_buffer.allocation.take().unwrap(),
                );
            }
        }
    }
}
