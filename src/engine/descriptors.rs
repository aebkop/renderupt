use erupt::vk;

use super::device::Physical;
pub struct Descriptors {
    pub global_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
}
impl Descriptors {
    pub fn new(physical: &Physical) -> Descriptors {
        let cam_buff_binding = vk::DescriptorSetLayoutBindingBuilder::new()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let bindings = vec![cam_buff_binding];
        let set_info = vk::DescriptorSetLayoutCreateInfoBuilder::new()
            .bindings(&bindings)
            .flags(vk::DescriptorSetLayoutCreateFlags::empty());
        let set = unsafe {
            physical
                .device
                .create_descriptor_set_layout(&set_info, None, None)
        }
        .unwrap();

        let sizes: Vec<vk::DescriptorPoolSizeBuilder> = vec![
            vk::DescriptorPoolSizeBuilder::new()
                ._type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(10);
            10
        ];

        let pool_info = vk::DescriptorPoolCreateInfoBuilder::new()
            .max_sets(10)
            .pool_sizes(&sizes);

        let descriptor_pool = unsafe {
            physical
                .device
                .create_descriptor_pool(&pool_info, None, None)
        }
        .unwrap();

        return {
            Descriptors {
                global_set_layout: set,
                descriptor_pool: descriptor_pool,
            }
        };
    }

    pub fn cleanup(&mut self, physical: &mut Physical) {
        unsafe {
            println!("PLEASE SAY WE GET HERE");
            physical
            .device
            .destroy_descriptor_pool(Some(self.descriptor_pool), None);
            physical
                .device
                .destroy_descriptor_set_layout(Some(self.global_set_layout), None);
        }
    }
}
