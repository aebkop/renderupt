use erupt::{vk::{self}};

use super::device::Physical;

pub struct SyncStructs {
    pub semaphores: Vec<vk::Semaphore>,
    pub fences: Vec<vk::Fence>
}

impl SyncStructs {
    pub fn new(physical: &Physical) -> Self {
        let fence_info = vk::FenceCreateInfoBuilder::new().flags(vk::FenceCreateFlags::SIGNALED);
        //we want to create the fence with the Create Signaled flag, so we can wait on it before using it on a GPU command (for the first frame)

        //don't need any info for the semaphore
        let mut semaphore_create_info = vk::SemaphoreCreateInfoBuilder::new();

        let render_semaphore =
            unsafe { physical.device.create_semaphore(&semaphore_create_info, None, None) }.unwrap();
        let present_semaphore =
            unsafe {  physical.device.create_semaphore(&semaphore_create_info, None, None) }.unwrap();

        let render_fence = unsafe {  physical.device.create_fence(&fence_info, None, None) }.unwrap();

        SyncStructs {
            semaphores: vec![render_semaphore, present_semaphore],
            fences: vec![render_fence]        
        }

    }

    pub fn cleanup(&mut self, physical: &Physical) {
        unsafe { 
        for semaphore in &self.semaphores {
            physical.device.destroy_semaphore(Some(*semaphore), None);
        }
        for fence in &self.fences {
            physical.device.destroy_fence(Some(*fence), None);
        }
    }}
}