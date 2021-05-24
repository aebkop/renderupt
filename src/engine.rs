mod mesh;
mod device;
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

use crate::engine::{device::Physical, mesh::{Vertex, Verticies}};

const LAYER_KHRONOS_VALIDATION: *const c_char = cstr!("VK_LAYER_KHRONOS_validation");
const VALIDATION_LAYERS_WANTED: bool = true;
const FRAG: &[u32] = include_glsl!("src/shaders/colored-triangle.frag", kind: frag);
const TRIMESH: &[u32] = include_glsl!("src/shaders/trimesh.vert");

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
        //vulkan initalization

        let mut physical = Physical::new(&window);
        
        let mut image_count = physical.surface_caps.min_image_count + 1;
        if physical.surface_caps.max_image_count > 0 && image_count > physical.surface_caps.max_image_count {
            image_count = physical.surface_caps.max_image_count;
        }

        let swapchain_info = vk::SwapchainCreateInfoKHRBuilder::new()
            .surface(physical.surface)
            .min_image_count(image_count)
            .image_format(physical.format.format)
            .image_color_space(physical.format.color_space)
            .image_extent(physical.surface_caps.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(physical.surface_caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagBitsKHR::OPAQUE_KHR)
            .present_mode(vk::PresentModeKHR::FIFO_RELAXED_KHR)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());

        let swapchain =
            unsafe { physical.device.create_swapchain_khr(&swapchain_info, None, None) }.unwrap();
        let swapchain_images = unsafe { physical.device.get_swapchain_images_khr(swapchain, None) }.unwrap();

        // https://vulkan-tutorial.com/Drawing_a_triangle/Presentation/Image_views
        let swapchain_image_views: Vec<_> = swapchain_images
            .iter()
            .map(|swapchain_image| {
                let image_view_info = vk::ImageViewCreateInfoBuilder::new()
                    .image(*swapchain_image)
                    .view_type(vk::ImageViewType::_2D)
                    .format(physical.format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(
                        vk::ImageSubresourceRangeBuilder::new()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1)
                            .build(),
                    );
                unsafe { physical.device.create_image_view(&image_view_info, None, None) }.unwrap()
            })
            .collect();
        let extent_3d = vk::Extent3DBuilder::new()
            .width(physical.surface_caps.current_extent.width)
            .height(physical.surface_caps.current_extent.height)
            .depth(1);
        
        //create depth images
        let image_create_info = vk::ImageCreateInfoBuilder::new() 
            .image_type(vk::ImageType::_2D)
            .format(vk::Format::D32_SFLOAT)
            .extent(extent_3d.build())
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlagBits::_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT);
        
         let image = unsafe { physical.device.create_image(&image_create_info, None, None).unwrap() }; 
        let mem_requirements = unsafe { physical.device.get_image_memory_requirements(image, None) }; 
        let alloc_request = gpu_alloc::Request {
            size: mem_requirements.size,
            align_mask: mem_requirements.alignment,
            usage: gpu_alloc::UsageFlags::FAST_DEVICE_ACCESS,
            memory_types: mem_requirements.memory_type_bits,
        };
        let mut block = unsafe { physical.allocator.alloc(EruptMemoryDevice::wrap(&physical.device), alloc_request) }.unwrap();

        unsafe {
            physical.device.bind_image_memory(image,*block.memory(),0).unwrap();
        }
        
        let image_view_create_info = vk::ImageViewCreateInfoBuilder::new()
            .view_type(vk::ImageViewType::_2D)
            .image(image)
            .format(vk::Format::D32_SFLOAT)
            .subresource_range(*vk::ImageSubresourceRangeBuilder::new().base_mip_level(0).level_count(1).base_array_layer(0).layer_count(1).aspect_mask(vk::ImageAspectFlags::DEPTH));

        let depth_image_view = unsafe { physical.device.create_image_view(&image_view_create_info, None, None)}.unwrap();
        
        
        let mut command_pool_info =
            vk::CommandPoolCreateInfoBuilder::new().queue_family_index(physical.graphics_queue_family).flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool =
            unsafe { physical.device.create_command_pool(&command_pool_info, None, None) }.unwrap();

        let command_buffer_info = vk::CommandBufferAllocateInfoBuilder::new()
            .command_pool(command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffer =
            unsafe { physical.device.allocate_command_buffers(&command_buffer_info) }.unwrap();

        let color_attachment = AttachmentDescription2Builder::new()
            .format(physical.format.format)
            .samples(vk::SampleCountFlagBits::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment_ref = vk::AttachmentReference2Builder::new()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let depth_attachment = vk::AttachmentDescription2Builder::new()
            .flags(vk::AttachmentDescriptionFlags::empty())
            .format(vk::Format::D32_SFLOAT)
            .samples(vk::SampleCountFlagBits::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::CLEAR)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        let depth_attachment_ref = vk::AttachmentReference2Builder::new()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);


        let color_attach_slice = &[color_attachment_ref];

        let mut subpass = SubpassDescription2Builder::new()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(color_attach_slice)
            .depth_stencil_attachment(&depth_attachment_ref);

        let attachments = [color_attachment,depth_attachment];
        let subpasses = [subpass];
        let mut render_pass_info = vk::RenderPassCreateInfo2Builder::new()
            .attachments(&attachments)
            .subpasses(&subpasses);            

        let render_pass =
            unsafe { physical.device.create_render_pass2(&render_pass_info, None, None) }.unwrap();

        //TODO Clean this up
        let framebuffers: Vec<_> = swapchain_image_views
            .iter()
            .map(|image_view| {
                let attachments = vec![*image_view, depth_image_view];
                let framebuffer_info = vk::FramebufferCreateInfoBuilder::new()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(physical.surface_caps.current_extent.width)
                    .height(physical.surface_caps.current_extent.height)
                    .layers(1);

                unsafe { physical.device.create_framebuffer(&framebuffer_info, None, None) }.unwrap()
            })
            .collect();

        let fence_info = vk::FenceCreateInfoBuilder::new().flags(vk::FenceCreateFlags::SIGNALED);
        //we want to create the fence with the Create Signaled flag, so we can wait on it before using it on a GPU command (for the first frame)

        //don't need any info for the semaphore
        let mut semaphore_create_info = vk::SemaphoreCreateInfoBuilder::new();

        let render_semaphore =
            unsafe { physical.device.create_semaphore(&semaphore_create_info, None, None) }.unwrap();
        let present_semaphore =
            unsafe {  physical.device.create_semaphore(&semaphore_create_info, None, None) }.unwrap();

        let mut render_fence: Vec<vk::Fence> = Vec::new();
        render_fence.push(unsafe {  physical.device.create_fence(&fence_info, None, None) }.unwrap());

        //Pipeline starts here
        //Shader Modules
        let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(FRAG);
        let frag_module = unsafe {  physical.device.create_shader_module(&module_info, None, None) }.unwrap();
        let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(TRIMESH);
        let entry_point = CString::new("main").unwrap();
        let tri_mesh =
            unsafe {  physical.device.create_shader_module(&module_info, None, None) }.unwrap();

    
        let shader_stages = vec![
            vk::PipelineShaderStageCreateInfoBuilder::new()
                .stage(vk::ShaderStageFlagBits::VERTEX)
                .module(tri_mesh)
                .name(&entry_point),
            vk::PipelineShaderStageCreateInfoBuilder::new()
                .stage(vk::ShaderStageFlagBits::FRAGMENT)
                .module(frag_module)
                .name(&entry_point),
        ];
        //like openGL VAO, not using it atm
        let vertex_desc = mesh::VertexDesc::new();
        let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
            .vertex_attribute_descriptions(&vertex_desc.attributes)
            .vertex_binding_descriptions(&vertex_desc.bindings);

        //what sort of topology drawn e.g triangles or lines or whatever
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let rasterizer = vk::PipelineRasterizationStateCreateInfoBuilder::new()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let multisampling = vk::PipelineMultisampleStateCreateInfoBuilder::new()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlagBits::_1);

        let color_blend_attachments = vec![vk::PipelineColorBlendAttachmentStateBuilder::new()
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .blend_enable(false)];
        let color_blending = vk::PipelineColorBlendStateCreateInfoBuilder::new()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);

        let viewports = vec![vk::ViewportBuilder::new()
            .x(0.0)
            .y(0.0)
            .width(physical.surface_caps.current_extent.width as f32)
            .height(physical.surface_caps.current_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];
        let scissors = vec![vk::Rect2DBuilder::new()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(physical.surface_caps.current_extent)];
        let viewport_state = vk::PipelineViewportStateCreateInfoBuilder::new()
            .viewports(&viewports)
            .scissors(&scissors);

        let pipeline_depth_stencil_info = vk::PipelineDepthStencilStateCreateInfoBuilder::new()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(false);
        
        let push_constant = [vk::PushConstantRangeBuilder::new()
            .offset(0)
            .size(size_of::<[f32; 16]>() as u32)
            .stage_flags(vk::ShaderStageFlags::VERTEX)];

        let pipeline_layout_info = vk::PipelineLayoutCreateInfoBuilder::new()
            .push_constant_ranges(&push_constant);
        let pipeline_layout =
            unsafe { physical.device.create_pipeline_layout(&pipeline_layout_info, None, None) }.unwrap();

        let pipeline_infos = vec![
            vk::GraphicsPipelineCreateInfoBuilder::new()
                //Colored Triangle
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input)
                .input_assembly_state(&input_assembly)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterizer)
                .multisample_state(&multisampling)
                .color_blend_state(&color_blending)
                .layout(pipeline_layout)
                .render_pass(render_pass)
                .depth_stencil_state(&pipeline_depth_stencil_info)
                .subpass(0),
        ];

        let pipelines =
            unsafe { physical.device.create_graphics_pipelines(None, &pipeline_infos, None) }.unwrap();

        //delete shader modules now.
        unsafe { 
            physical.device.destroy_shader_module(Some(frag_module),None);
            physical.device.destroy_shader_module(Some(tri_mesh),None);
        }

        
/*           let triangle_data =  vec![
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
 */

/*         let triangle_data =  vec![
            mesh::Vertex {
                pos: Vector3::new(1.0, 1.0, 0.0),
                color: Vector3::new(0.0,1.0,0.0),
                normal: Vector3::new(0.0,0.0,0.0)
            
            },
            mesh::Vertex {
                pos: Vector3::new(-1.0, 1.0, 0.0),
                color: Vector3::new(0.0,1.0,0.0),
                normal: Vector3::new(0.0,0.0,0.0)
            }, 
            mesh::Vertex {
                pos: Vector3::new(0.0, -1.0, 0.0),
                color: Vector3::new(0.0,1.0,0.0),
                normal: Vector3::new(0.0,0.0,0.0)
            },
        ];
 */    
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
            pipelines,
            pipeline_layout,
            present_semaphore,
            render_semaphore,
            render_fence,
            render_pass,
            framebuffers,
            command_pool,
            command_buffer: command_buffer,
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
            _swapchain: swapchain,
            _swapchain_images: swapchain_images,
            _swapchain_image_views: swapchain_image_views,
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
        let eye    = na::Point3::<f32>::new(0.0, -1.0, 1.0);
        let target = na::Point3::<f32>::new(1.0, 0.0, 0.0);
        let view   = na::Isometry3::<f32>::look_at_rh(&eye, &target, &Vector3::y());
        let model      = na::Isometry3::<f32>::new(Vector3::zeros(), Vector3::y() * f32::to_radians(framenumber as f32 * 0.4));
        let  projection = na::Perspective3::<f32>::new(self.surface_caps.current_extent.width as f32 / self.surface_caps.current_extent.height as f32, 3.14 / 2.0, 0.1, 200.0).into_inner();
        let model_view_projection:na::Matrix4<f32> = projection * (view * model).to_homogeneous();
	
                
        let mesh_push_constants = mesh::push_mesh_constants {
            data: na::Vector3::new(0.0, 0.0, 0.0),
            render_matrix: model_view_projection
        };




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
