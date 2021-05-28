use super::{device::Physical, swapchain::Swapchain};


use erupt::{vk::{self}};
use gpu_alloc_erupt::EruptMemoryDevice;

pub struct RenderPass {
    pub framebuffers: Vec<vk::Framebuffer>,
    pub render_pass: vk::RenderPass

}

impl RenderPass {
    pub fn new(physical: &mut Physical, swapchain: &Swapchain) -> Self {
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

    let color_attachment = vk::AttachmentDescription2Builder::new()
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

let mut subpass = vk::SubpassDescription2Builder::new()
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
let framebuffers: Vec<_> = swapchain.image_views
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

    RenderPass {
        framebuffers,
        render_pass
    }

    }
    pub fn cleanup(&mut self, physical: &Physical) {
        unsafe { 
        for framebuffer in self.framebuffers.iter() {
            physical.device.destroy_framebuffer(Some(*framebuffer), None);
        }
        physical.device.destroy_render_pass(Some(self.render_pass), None)

    }}
}