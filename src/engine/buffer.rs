
use super::{device::Physical, mesh::AllocatedBuffer};

use erupt::vk::{self, DeviceMemory};
use gpu_alloc::{MemoryBlock, Request, UsageFlags};
use gpu_alloc_erupt::EruptMemoryDevice;


pub fn create_buffer(physical: &mut Physical, alloc_size: u64, usage: vk::BufferUsageFlags, memory_usage: gpu_alloc::UsageFlags) -> AllocatedBuffer {
    let buffer_info = vk::BufferCreateInfoBuilder::new()
    .size(alloc_size)
    .usage(usage);

    let mut block = unsafe {
        physical.allocator.alloc(
                EruptMemoryDevice::wrap(&physical.device),
                Request {
                    size: alloc_size as u64,
                    align_mask: 1,
                    usage: memory_usage,
                    memory_types: !0,
                },
            )
        }.unwrap();
                
    let buffer = unsafe { physical.device.create_buffer(&buffer_info, None, None) }.unwrap();

    unsafe {
        physical.device.bind_buffer_memory(buffer, *block.memory(), 0).unwrap();
    }

    AllocatedBuffer {
        buffer,
        allocation: Some(block)
    }





}