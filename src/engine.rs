mod mesh;
mod device;
mod swapchain;
mod renderpass;
mod commands;
mod sync;
mod pipeline;
mod scene;
extern crate nalgebra as na;
extern crate nalgebra_glm as glm;


use bytemuck::bytes_of;
use erupt::{DeviceLoader, EntryLoader, ExtendableFrom, InstanceLoader, cstr, utils::surface, vk::{self, AttachmentDescription2Builder, CommandPoolCreateFlags, DebugUtilsMessengerEXT, DeviceMemory, FN_CREATE_DIRECT_FB_SURFACE_EXT, StructureType, SubpassDescription2Builder}};
use nalgebra::Vector3;
use core::{f64};
use std::{ffi::{c_void, CStr, CString}, mem::{size_of, size_of_val}, os::raw::c_char, ptr::addr_of_mut};
use vk_shader_macros::include_glsl;

use gpu_alloc::{Config, GpuAllocator, Request, UsageFlags};
use gpu_alloc_erupt::{device_properties as device_properties_alloc, EruptMemoryDevice};

use winit::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::engine::{commands::Command, device::Physical, mesh::{Vertex}, renderpass::RenderPass, swapchain::Swapchain, sync::SyncStructs};

use self::{mesh::Mesh, pipeline::PipelineStruct, scene::Scene};

const LAYER_KHRONOS_VALIDATION: *const c_char = cstr!("VK_LAYER_KHRONOS_validation");
const VALIDATION_LAYERS_WANTED: bool = true;

//debug_callback for the validation layers
unsafe extern "system" fn debug_callback(
    _message_severity: vk::DebugUtilsMessageSeverityFlagBitsEXT,
    _message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    eprintln!(
        "{}",
        CStr::from_ptr((*p_callback_data).p_message).to_string_lossy()
    );

    vk::FALSE
}

//This needs to be in order of what needs to be destroyed first - The Drop trait destroys them in order of declaration, i.e the first item is destroyed first.
pub struct VulkanApp {
    scene: Scene,
    sync: SyncStructs,
    command: Command,
    render_pass: RenderPass,
    swapchain: Swapchain,
    physical: Physical
}

impl VulkanApp {
    pub fn new(window : &Window) -> Self {
        //window/wi
        //this needs to be mut because device and the allocator gets mutated when doing commands 
        let mut physical = Physical::new(window);

        let swapchain = Swapchain::new(&physical);

        let render_pass = RenderPass::new(&mut physical, &swapchain);
        
        let command = Command::new(&physical);

        let sync = SyncStructs::new(&physical);

        let pipeline = PipelineStruct::new(&physical, &render_pass);

        let mut scene = Scene::new();
        let mesh = Mesh::new(std::path::Path::new("D:/rustprogramming/vulkan-guide/vkguide-erupt/src/assets/monkey_smooth.obj"), &mut physical);
        let axisangle = Vector3::y() * std::f32::consts::FRAC_PI_2;
        let mesh_matrix: na::Isometry3<f32> = na::Isometry3::new(Vector3::x(), axisangle);
        scene.add_render_object_with_mesh_material(mesh, "monkey", scene::Material{pipeline}, "default", mesh_matrix);

        let triangle_data =  vec![
            Vertex {
                pos: [0.5,-0.5,-0.5],
                color: [0.0,1.0,0.0],
                normal: [0.0,0.0,0.0]
            },
            Vertex {
                pos: [0.5,-0.5,0.5],
                color: [0.0,1.0,0.0],
                normal: [0.0,0.0,0.0]
            },
            Vertex {
                pos: [-0.5,-0.5,0.5],
                color: [0.0,1.0,0.0],
                normal: [0.0,0.0,0.0]
            },
            Vertex {
                pos: [-0.5,-0.5,-0.5],
                color: [0.0,1.0,0.0],
                normal: [0.0,0.0,0.0]
            },
            Vertex {
                pos: [0.5,0.5,-0.5],
                color: [0.0,1.0,0.0],
                normal: [0.0,0.0,0.0]
            },
            Vertex {
                pos: [0.5,0.5,0.5],
                color: [0.0,1.0,0.0],
                normal: [0.0,0.0,0.0]
            },
            Vertex {
                pos: [-0.5,0.5,0.5],
                color: [0.0,1.0,0.0],
                normal: [0.0,0.0,0.0]
            },
            Vertex {
                pos: [-0.5,0.5,-0.5],
                color: [0.0,1.0,0.0],
                normal: [0.0,0.0,0.0]
            }
        ];
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
            }.unwrap();
                    
        unsafe {
            block.write_bytes(EruptMemoryDevice::wrap(&physical.device), 0, data).unwrap();
        }
        
        let buffer = unsafe { physical.device.create_buffer(&buffer_info, None, None) }.unwrap();

        unsafe {
            physical.device.bind_buffer_memory(buffer, *block.memory(), 0).unwrap();
        }

        let allocated_buff = mesh::allocated_buffer {
            buffer,
            allocation: Some(block)
        };

        let triangle = mesh::Mesh {
            verticies:  triangle_data,
            vertex_buffer: (
                allocated_buff
            ),

        };

        let cube_matrix: na::Isometry3<f32> = na::Isometry3::new(Vector3::new(5.0,0.0,0.0), na::zero());
        scene.add_render_object_with_mesh(triangle, "cube", "default", cube_matrix);

        let cube = Mesh::new(std::path::Path::new("D:/rustprogramming/vulkan-guide/vkguide-erupt/src/assets/teapot.obj"), &mut physical);
        let test    : na::Isometry3<f32> = na::Isometry3::new(Vector3::new(10.0,-3.0,3.0), na::zero());
        scene.add_render_object_with_mesh( cube, "teapot", "default", test);

        





            
        VulkanApp {
            scene,
            sync,
            command,
            render_pass,
            swapchain,
            physical,
        }
    }
    
    fn draw_objects(&mut self, framenumber: i64)  {
                //compute push constant
        let eye    = na::Point3::<f32>::new(0.0, 0.0, 2.0);
        let target = na::Point3::<f32>::new(1.0, 0.0, 0.0);
        let view   = na::Isometry3::<f32>::look_at_rh(&eye, &target, &Vector3::y());
        let camera_angle      = na::Isometry3::<f32>::new(Vector3::zeros(), Vector3::y() * f32::to_radians(framenumber as f32 * 0.4));
        let projection = na::Perspective3::<f32>::new(self.physical.surface_caps.current_extent.width as f32 / self.physical.surface_caps.current_extent.height as f32, 3.14 / 1.5, 0.1, 200.0).into_inner();
        let material = self.scene.materials.get("default").unwrap();
        unsafe {
            self.physical.device.cmd_bind_pipeline(self.command.buffer[0], vk::PipelineBindPoint::GRAPHICS, material.pipeline.pipelines[0]);
            }
        for (a,b,c) in self.scene.objects.iter() {
            let mesh = self.scene.meshes.get(a).unwrap();
            let offset: u64 = 0;
            let model_view_projection:na::Matrix4<f32> = projection * (camera_angle * view * c).to_homogeneous();

            unsafe {
                self.physical.device.cmd_push_constants(self.command.buffer[0], material.pipeline.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, size_of_val(model_view_projection.as_slice()) as u32, model_view_projection.as_slice().as_ptr() as *mut c_void);
                self.physical.device.cmd_bind_vertex_buffers(self.command.buffer[0], 0, &[mesh.vertex_buffer.buffer], &[offset]);
                self.physical.device.cmd_draw(self.command.buffer[0], mesh.verticies.len() as u32, 1, 0, 0);
            }

            

        }
    }

    //Present semaphore - 0
    //render - 1
        
        

    pub fn draw(&mut self, framenumber: i64) {
        unsafe {
            self.physical.device
                .wait_for_fences(&self.sync.fences, false, u64::MAX)
                .unwrap();
            self.physical.device.reset_fences(&self.sync.fences)
        }
        .unwrap();
        let swapchain_image_index = unsafe {
            self.physical.device.acquire_next_image_khr(
                self.swapchain.swapchain,
                u64::MAX,
                Some(self.sync.semaphores[0]),
                Some(vk::Fence::null()),
                None,
            )
        }
        .unwrap();
        //reset command buffer and start it again
        unsafe {
            self.physical.device.reset_command_buffer(
                self.command.buffer[0],
                Some(vk::CommandBufferResetFlags::RELEASE_RESOURCES),
            )
        }
        .unwrap();
        
        let cmd_begin_info = vk::CommandBufferBeginInfoBuilder::new()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.physical.device
                .begin_command_buffer(self.command.buffer[0], &cmd_begin_info)
                .unwrap();
        }
        //make a clear-color from frame number. This will flash with a 120*pi frame period.
        let flashdiv120 = framenumber as f32 / 120 as f32;
        let flash: f32 = flashdiv120.sin().abs();
        let clear_value = vk::ClearValue {
            color: {
                vk::ClearColorValue {
                    float32: [0.0, 0.0, flash, 1.0],
                }
            },
        };
        let depth_stencil = vk::ClearDepthStencilValueBuilder::new().depth(1.0);
        let depth_clear = vk::ClearValue {
            depth_stencil: *depth_stencil 
        };
        let clear_values = vec![clear_value, depth_clear];

        //start the main renderpass
        let rp_info = vk::RenderPassBeginInfoBuilder::new()
            .render_pass(self.render_pass.render_pass)
            .framebuffer(self.render_pass.framebuffers[swapchain_image_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.physical.surface_caps.current_extent,
            })
            .clear_values(&clear_values);
        unsafe {
            self.physical.device.cmd_begin_render_pass(
                self.command.buffer[0],
                &rp_info,
                vk::SubpassContents::INLINE,
            )
        };

        self.draw_objects(framenumber);
            	
        unsafe {
            //end renderpass
            self.physical.device.cmd_end_render_pass(self.command.buffer[0]);
            self.physical.device
                .end_command_buffer(self.command.buffer[0])
                .unwrap();
        }

        let present_semaphores = vec![self.sync.semaphores[0]];
        let render_semaphores = vec![self.sync.semaphores[1]];
        let swapchains = vec![self.swapchain.swapchain];
        let swapchain_index_indices = vec![swapchain_image_index];

        //we can now submit the render pass to the GPU
        let submit_info = vk::SubmitInfoBuilder::new()
            .wait_semaphores(&present_semaphores)
            .signal_semaphores(&render_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&self.command.buffer);
        let submit = vec![submit_info];
        unsafe {
            self.physical.device
                .queue_submit(self.physical.graphics_queue, &submit, Some(self.sync.fences[0]))
        }
        .unwrap();

        let present_info = vk::PresentInfoKHRBuilder::new()
            .wait_semaphores(&render_semaphores)
            .swapchains(&swapchains)
            .image_indices(&swapchain_index_indices);
        unsafe {
            self.physical.device
                .queue_present_khr(self.physical.graphics_queue, &present_info)
        }
        .unwrap();
    }

}
//Instead of a cleanup function the drop trait is used which runs automatically after the value is no longer needed.
impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {

            self.physical.device.device_wait_idle().unwrap();

            self.scene.cleanup(&mut self.physical);
            
            self.sync.cleanup(&self.physical);

            self.command.cleanup(&self.physical);

            self.render_pass.cleanup(&mut self.physical);

            self.swapchain.cleanup(&self.physical);

            self.physical.cleanup();

            println!("exited");
        }
    }
}
