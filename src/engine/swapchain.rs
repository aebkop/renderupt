use super::device::Physical;

use erupt::vk::{self};

pub struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

impl Swapchain {
    pub fn new(physical: &Physical) -> Self {
        let mut image_count = physical.surface_caps.min_image_count + 1;
        if physical.surface_caps.max_image_count > 0
            && image_count > physical.surface_caps.max_image_count
        {
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

        let swapchain = unsafe {
            physical
                .device
                .create_swapchain_khr(&swapchain_info, None, None)
        }
        .unwrap();
        let swapchain_images =
            unsafe { physical.device.get_swapchain_images_khr(swapchain, None) }.unwrap();

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
                unsafe {
                    physical
                        .device
                        .create_image_view(&image_view_info, None, None)
                }
                .unwrap()
            })
            .collect();

        Swapchain {
            swapchain,
            images: swapchain_images,
            image_views: swapchain_image_views,
        }
    }
    pub fn cleanup(&mut self, physical: &Physical) {
        unsafe {
            physical.device.destroy_swapchain_khr(Some(self.swapchain), None);
            for &image_view in self.image_views.iter() {
                physical.device.destroy_image_view(Some(image_view), None);
            }
        }
    }
}
