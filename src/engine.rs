mod mesh;
mod device;
mod swapchain;
mod renderpass;
mod commands;
mod sync;
mod pipeline;
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

use crate::engine::{commands::Command, device::Physical, mesh::{Vertex, Verticies}, renderpass::RenderPass, swapchain::Swapchain, sync::SyncStructs};

use self::pipeline::PipelineStruct;

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
    mesh: mesh::Mesh,
    surface_caps: vk::SurfaceCapabilitiesKHR,
    pipelines: Vec<vk::Pipeline>,
    pipeline_layout: vk::PipelineLayout,
    present_semaphore: vk::Semaphore,
    render_semaphore: vk::Semaphore,
    render_fence: Vec<vk::Fence>,
    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    graphics_queue: vk::Queue,
    graphics_queue_family: u32,
    command_pool: vk::CommandPool,
    command_buffer: Vec<vk::CommandBuffer>,
    _swapchain_image_views: Vec<vk::ImageView>,
    _swapchain_images: Vec<vk::Image>,
    _swapchain: vk::SwapchainKHR,
    allocator: GpuAllocator<DeviceMemory>,
    device: DeviceLoader,
    chosen_gpu: vk::PhysicalDevice,
    instance: InstanceLoader,
    surface: vk::SurfaceKHR,
    _entry: EntryLoader<libloading::Library>,
    messenger: DebugUtilsMessengerEXT,
    window: Window,
    _isinit: bool,
}

impl VulkanApp {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        //window/winit initalization
        let window = WindowBuilder::new()
            .with_title("test")
            .with_resizable(true)
            .build(&event_loop)
            .unwrap();

        //this needs to be mut because device and the allocator gets mutated when doing commands 
        let mut physical = Physical::new(&window);

        let swapchain = Swapchain::new(&physical);

        let render_pass = RenderPass::new(&mut physical, &swapchain);
        
        let command = Command::new(&physical);

        let sync = SyncStructs::new(&physical);

        let pipeline = PipelineStruct::new(&physical, &render_pass);
        
    
       let triangle_data = mesh::test(std::path::Path::new("D:/rustprogramming/vulkan-guide/vkguide-erupt/src/assets/teapot.obj"));
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
            allocation: block
        };

        let mesh = mesh::Mesh {
            verticies:  triangle_data,
            vertex_buffer: (
                allocated_buff
            ),

        };

            
        VulkanApp {
            mesh,
            surface_caps: physical.surface_caps,
            pipelines: pipeline.pipelines,
            pipeline_layout: pipeline.pipeline_layout,
            present_semaphore: sync.semaphores[0],
            render_semaphore: sync.semaphores[1],
            render_fence: sync.fences,
            render_pass: render_pass.render_pass,
            framebuffers: render_pass.framebuffers,
            command_pool: command.pool ,
            command_buffer: command.buffer,
            graphics_queue: physical.graphics_queue,
            graphics_queue_family: physical.graphics_queue_family,
            window,
            messenger: physical.messenger,
            _entry: physical.entry,
            instance: physical.instance,
            surface: physical.surface,
            chosen_gpu: physical.physical_device,
            allocator: physical.allocator,
            device: physical.device,
            _swapchain: swapchain.swapchain,
            _swapchain_images: swapchain.images,
            _swapchain_image_views: swapchain.image_views,
            _isinit: true,
        }
    }
    
    fn load_meshes(&mut self)  {
    }

    fn draw(&mut self, framenumber: i64, selected_shader: bool) {
        unsafe {
            self.device
                .wait_for_fences(&self.render_fence, false, u64::MAX)
                .unwrap();
            self.device.reset_fences(&self.render_fence)
        }
        .unwrap();
        let swapchain_image_index = unsafe {
            self.device.acquire_next_image_khr(
                self._swapchain,
                u64::MAX,
                Some(self.present_semaphore),
                Some(vk::Fence::null()),
                None,
            )
        }
        .unwrap();
        //reset command buffer and start it again
        unsafe {
            self.device.reset_command_buffer(
                self.command_buffer[0],
                Some(vk::CommandBufferResetFlags::RELEASE_RESOURCES),
            )
        }
        .unwrap();
        
        let cmd_begin_info = vk::CommandBufferBeginInfoBuilder::new()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .begin_command_buffer(self.command_buffer[0], &cmd_begin_info)
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
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[swapchain_image_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.surface_caps.current_extent,
            })
            .clear_values(&clear_values);
        unsafe {
            self.device.cmd_begin_render_pass(
                self.command_buffer[0],
                &rp_info,
                vk::SubpassContents::INLINE,
            )
        };
            
        unsafe {
            self.device.cmd_bind_pipeline(
                self.command_buffer[0],
                vk::PipelineBindPoint::GRAPHICS,
                self.pipelines[0],
            );
            let offset: u64 = 0;
            self.device.cmd_bind_vertex_buffers(self.command_buffer[0], 0, &[self.mesh.vertex_buffer.buffer], &[offset]);
        };  
        //compute push constant
        let eye    = na::Point3::<f32>::new(0.0, 0.0, 200.0);
        let target = na::Point3::<f32>::new(1.0, 0.0, 0.0);
        let view   = na::Isometry3::<f32>::look_at_rh(&eye, &target, &Vector3::y());
        let model      = na::Isometry3::<f32>::new(Vector3::zeros(), Vector3::y() * f32::to_radians(framenumber as f32 * 0.4));
        let  projection = na::Perspective3::<f32>::new(self.surface_caps.current_extent.width as f32 / self.surface_caps.current_extent.height as f32, 3.14 / 2.0, 0.1, 200.0).into_inner();
        let model_view_projection:na::Matrix4<f32> = projection * (view * model).to_homogeneous();
	
        unsafe {
            self.device.cmd_push_constants(self.command_buffer[0], self.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, size_of_val(model_view_projection.as_slice()) as u32, model_view_projection.as_slice().as_ptr() as *mut c_void 
    );
            self.device.cmd_draw(self.command_buffer[0], self.mesh.verticies.len() as u32, 1, 0, 0);

            //end renderpass
            self.device.cmd_end_render_pass(self.command_buffer[0]);
            self.device
                .end_command_buffer(self.command_buffer[0])
                .unwrap();
        }

        let present_semaphores = vec![self.present_semaphore];
        let render_semaphores = vec![self.render_semaphore];
        let swapchains = vec![self._swapchain];
        let swapchain_index_indices = vec![swapchain_image_index];

        //we can now submit the render pass to the GPU
        let submit_info = vk::SubmitInfoBuilder::new()
            .wait_semaphores(&present_semaphores)
            .signal_semaphores(&render_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&self.command_buffer);
        let submit = vec![submit_info];
        unsafe {
            self.device
                .queue_submit(self.graphics_queue, &submit, Some(self.render_fence[0]))
        }
        .unwrap();

        let present_info = vk::PresentInfoKHRBuilder::new()
            .wait_semaphores(&render_semaphores)
            .swapchains(&swapchains)
            .image_indices(&swapchain_index_indices);
        unsafe {
            self.device
                .queue_present_khr(self.graphics_queue, &present_info)
        }
        .unwrap();
    }

    pub fn run(mut self, event_loop: EventLoop<()>) {
        //state variables
        let mut framenumber: i64 = 0;
        let mut selected_shader: bool = true;


        event_loop.run(move |event, _, control_flow| match event {
            Event::NewEvents(StartCause::Init) => {
                *control_flow = ControlFlow::Poll;
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::Key(KeyboardInput {
                    virtual_keycode: Some(keycode),
                    state,
                    ..
                }) => match (keycode, state) {
                    (VirtualKeyCode::Escape, ElementState::Released) => {
                        *control_flow = ControlFlow::Exit
                    }
                    (VirtualKeyCode::Space, ElementState::Released) => {
                        selected_shader = !selected_shader;
                    }
                    _ => (),
                },
                _ => (),
            },
            Event::MainEventsCleared => {
                self.draw(framenumber, selected_shader);
                framenumber = framenumber + 1;
            }
            _ => (),
        });
    } 
}
//Instead of a cleanup function the drop trait is used which runs automatically after the value is no longer needed.
impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {

            self.device.device_wait_idle().unwrap();

            self.device
                .destroy_command_pool(Some(self.command_pool), None);
            self.device
                .destroy_swapchain_khr(Some(self._swapchain), None);
            self.device
                .destroy_render_pass(Some(self.render_pass), None);

            for &image_view in self._swapchain_image_views.iter() {
                self.device.destroy_image_view(Some(image_view), None);
            }

            for &framebuffer in self.framebuffers.iter() {
                self.device.destroy_framebuffer(Some(framebuffer), None);
            }

            for &pipeline in self.pipelines.iter() {
                self.device.destroy_pipeline(Some(pipeline), None);
            }

            self.device.destroy_pipeline_layout(Some(self.pipeline_layout), None);
            
            for &fence in self.render_fence.iter() {
                self.device.destroy_fence(Some(fence), None);
            }

            self.device.destroy_semaphore(Some(self.render_semaphore), None);
            self.device.destroy_semaphore(Some(self.present_semaphore), None);

            self.device.destroy_device(None);
            self.instance.destroy_surface_khr(Some(self.surface), None);
            if !self.messenger.is_null() {
                self.instance
                    .destroy_debug_utils_messenger_ext(Some(self.messenger), None);
            }
            self.instance.destroy_instance(None);
            println!("exited");
        }
    }
}
