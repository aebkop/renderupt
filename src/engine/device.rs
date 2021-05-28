const LAYER_KHRONOS_VALIDATION: *const c_char = cstr!("VK_LAYER_KHRONOS_validation");
const VALIDATION_LAYERS_WANTED: bool = true;

use std::{ffi::{CStr, CString, c_void}, os::raw::c_char};

use erupt::{DeviceLoader, EntryLoader, ExtendableFrom, InstanceLoader, cstr, utils::surface, vk::{self, DeviceMemory}};
use gpu_alloc::{Config, GpuAllocator, Request, UsageFlags};
use gpu_alloc_erupt::{device_properties as device_properties_alloc, EruptMemoryDevice};
use winit::window::Window;

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

pub struct Physical {
    pub surface_caps: vk::SurfaceCapabilitiesKHR,
    pub allocator: GpuAllocator<DeviceMemory>,
    pub format: vk::SurfaceFormatKHR,
    pub graphics_queue: vk::Queue,
    pub graphics_queue_family: u32,
    pub physical_device: vk::PhysicalDevice,
    pub device: DeviceLoader,
    pub messenger: vk::DebugUtilsMessengerEXT,
    pub surface: vk::SurfaceKHR,
    pub instance: InstanceLoader,
    pub entry: EntryLoader<libloading::Library>
}

impl Physical {
    pub fn new(window: &Window) -> Self {

        let entry = EntryLoader::new().unwrap();

        let application_name = CString::new("Renderupt").unwrap();
        let app_info = Box::new(
            vk::ApplicationInfoBuilder::new()
                .api_version(vk::make_version(1, 2, 0))
                .application_name(&application_name)
                .engine_version(vk::make_version(1, 1, 0))
        );

        //set up required extension + swapchain + validation
        let mut instance_extensions = surface::enumerate_required_extensions(window).unwrap();
        if VALIDATION_LAYERS_WANTED {
            instance_extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION_NAME);
        }

        let mut instance_layers = Vec::new();
        if VALIDATION_LAYERS_WANTED {
            instance_layers.push(LAYER_KHRONOS_VALIDATION);
        }

        // swapchian extension wanted as well
        let device_extensions = vec![vk::KHR_SWAPCHAIN_EXTENSION_NAME, vk::KHR_BUFFER_DEVICE_ADDRESS_EXTENSION_NAME];

        let mut device_layers = Vec::new();
        if VALIDATION_LAYERS_WANTED {
            device_layers.push(LAYER_KHRONOS_VALIDATION);
        }

        let instance_info = vk::InstanceCreateInfoBuilder::new()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .enabled_layer_names(&instance_layers);

        let instance =  InstanceLoader::new(&entry, &instance_info, None).unwrap();

        let messenger = if VALIDATION_LAYERS_WANTED {
            let messenger_info = vk::DebugUtilsMessengerCreateInfoEXTBuilder::new()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE_EXT
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING_EXT
                        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR_EXT,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL_EXT
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION_EXT
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE_EXT,
                )
                .pfn_user_callback(Some(debug_callback));

            unsafe { instance.create_debug_utils_messenger_ext(&messenger_info, None, None) }
                .unwrap()
        } else {
            Default::default()
        };

        //create a surface to draw on
        let surface = unsafe { surface::create_surface(&instance, window, None) }.unwrap();

        //get a device and queue
        let (physical_device, queue_family, format, present_mode, device_properties) =
            unsafe { instance.enumerate_physical_devices(None) }
                .unwrap()
                .into_iter()
                .filter_map(|physical_device| unsafe {
                    let queue_family = match instance
                        .get_physical_device_queue_family_properties(physical_device, None)
                        .into_iter()
                        .enumerate()
                        .position(|(i, queue_family_properties)| {
                            queue_family_properties
                                .queue_flags
                                .contains(vk::QueueFlags::GRAPHICS)
                                && instance
                                    .get_physical_device_surface_support_khr(
                                        physical_device,
                                        i as u32,
                                        surface,
                                        None,
                                    )
                                    .unwrap()
                        }) {
                        Some(queue_family) => queue_family as u32,
                        None => return None,
                    };

                    let formats = instance
                        .get_physical_device_surface_formats_khr(physical_device, surface, None)
                        .unwrap();
                    let format = match formats
                        .iter()
                        .find(|surface_format| {
                            surface_format.format == vk::Format::B8G8R8A8_SRGB
                                && surface_format.color_space
                                    == vk::ColorSpaceKHR::SRGB_NONLINEAR_KHR
                        })
                        .or_else(|| formats.get(0))
                    {
                        Some(surface_format) => *surface_format,
                        None => return None,
                    };

                    let present_mode = instance
                        .get_physical_device_surface_present_modes_khr(
                            physical_device,
                            surface,
                            None,
                        )
                        .unwrap()
                        .into_iter()
                        .find(|present_mode| present_mode == &vk::PresentModeKHR::MAILBOX_KHR)
                        .unwrap_or(vk::PresentModeKHR::FIFO_KHR);

                    let supported_device_extensions = instance
                        .enumerate_device_extension_properties(physical_device, None, None)
                        .unwrap();
                    let device_extensions_supported =
                        device_extensions.iter().all(|device_extension| {
                            let device_extension = CStr::from_ptr(*device_extension);

                            supported_device_extensions.iter().any(|properties| {
                                CStr::from_ptr(properties.extension_name.as_ptr())
                                    == device_extension
                            })
                        }); 

                    if !device_extensions_supported {
                        return None;
                    }

                    let device_properties =
                        instance.get_physical_device_properties(physical_device, None);
                    Some((
                        physical_device,
                        queue_family,
                        format,
                        present_mode,
                        device_properties,
                    ))
                })
                .max_by_key(|(_, _, _, _, properties)| match properties.device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 2,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                    _ => 0,
                })
                .expect("No suitable physical device found");

        println!("Using physical device: {:?}", unsafe {
            CStr::from_ptr(device_properties.device_name.as_ptr())
        });

        let queue_info = vec![vk::DeviceQueueCreateInfoBuilder::new()
            .queue_family_index(queue_family)
            .queue_priorities(&[1.0])];
        let mut features = vk::PhysicalDeviceFeaturesBuilder::new();
            
        
        let mut test2 = vk::PhysicalDeviceVulkan12FeaturesBuilder::new()
            .buffer_device_address(true);
                   
        let mut device_features2_builder =
            vk::PhysicalDeviceFeatures2Builder::new().extend_from(&mut test2);


        //set features and extensions enabled in the device
        let device_info = vk::DeviceCreateInfoBuilder::new()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extensions)
            .enabled_layer_names(&device_layers)
            .extend_from(&mut device_features2_builder);
            
        let device_properties_alloc = unsafe { device_properties_alloc(&instance, physical_device)}.unwrap();
        
        //finally have a device and queue
        let device =
            DeviceLoader::new(&instance, physical_device, &device_info, None).unwrap();
        let queue = unsafe { device.get_device_queue(queue_family, 0, None) };

    
        let config = Config::i_am_potato();

        let mut gpu_alloc = GpuAllocator::new(config, device_properties_alloc);

        //create a swapchain
        let surface_caps = unsafe {
                instance.get_physical_device_surface_capabilities_khr(physical_device, surface, None)
            }
                .unwrap();
        
        
        Physical {
            surface_caps,
            allocator: gpu_alloc,
            format,
            physical_device,
            graphics_queue_family: queue_family,
            graphics_queue: queue,
            device,
            messenger,
            surface,
            instance,
            entry,
            
        }
    }
    pub fn cleanup(&mut self) {
        unsafe { 
        self.allocator.cleanup(EruptMemoryDevice::wrap(&self.device));
        self.device.destroy_device(None);
        self.instance.destroy_surface_khr(Some(self.surface), None);
        if !self.messenger.is_null() {
            self.instance
                .destroy_debug_utils_messenger_ext(Some(self.messenger), None);
        }
        self.instance.destroy_instance(None);
    }}
}
