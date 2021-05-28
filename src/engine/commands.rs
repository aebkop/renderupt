
use erupt::{vk::{self}};
use super::{device::Physical};


pub struct Command {
    pub pool: vk::CommandPool,
    pub buffer: Vec<vk::CommandBuffer>
}

impl Command {
    pub fn new(physical: &Physical) -> Self {
    let command_pool_info =
        vk::CommandPoolCreateInfoBuilder::new().queue_family_index(physical.graphics_queue_family).flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
    let command_pool =
        unsafe { physical.device.create_command_pool(&command_pool_info, None, None) }.unwrap();

    let command_buffer_info = vk::CommandBufferAllocateInfoBuilder::new()
        .command_pool(command_pool)
        .command_buffer_count(1)
        .level(vk::CommandBufferLevel::PRIMARY);

    let command_buffer =
        unsafe { physical.device.allocate_command_buffers(&command_buffer_info) }.unwrap();
    
    Command {
        buffer: command_buffer,
        pool: command_pool
    }
    
    }
    pub fn cleanup(&mut self, physical: &Physical) {
        unsafe {
            physical.device.destroy_command_pool(Some(self.pool), None)
    }}
}

