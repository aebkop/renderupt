use erupt::vk;

use super::device::Physical;

pub struct Frame {
    pub present_semaphore: vk::Semaphore,
    pub render_semaphore: vk::Semaphore,
    pub render_fence: vk::Fence,
    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,
}

pub struct Frames {
    pub frames: Vec<Frame>,
}

impl Frames {
    pub fn new(frame_count: u32, physical: &mut Physical) -> Self {
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

            frames.push(Frame {
                present_semaphore,
                render_semaphore,
                render_fence,
                command_pool,
                command_buffer: command_buffer[0],
            })
        }
        Frames { frames }
    }
    pub fn cleanup(&mut self, physical: &mut Physical) {
        for frame in &self.frames {
            unsafe { 
            physical.device.destroy_semaphore(Some(frame.render_semaphore), None);
            physical.device.destroy_semaphore(Some(frame.present_semaphore), None);
            physical.device.destroy_fence(Some(frame.render_fence), None);
            physical.device.destroy_command_pool(Some(frame.command_pool), None);
            }
        }
    }
}
