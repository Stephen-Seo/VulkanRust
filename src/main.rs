mod ffi;

use std::collections::HashSet;
use std::ffi::{CStr, CString};

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;

#[cfg(debug_assertions)]
const ENABLE_VALIDATION_LAYERS: bool = true;
#[cfg(not(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = false;

const VALIDATION_LAYER_STR_0: &str = "VK_LAYER_KHRONOS_validation\x00";
const VALIDATION_LAYERS: [*const u8; 1] = [VALIDATION_LAYER_STR_0.as_ptr()];

const DYNAMIC_STATES: [ffi::VkDynamicState; 2] = [
    ffi::VkDynamicState_VK_DYNAMIC_STATE_VIEWPORT,
    ffi::VkDynamicState_VK_DYNAMIC_STATE_SCISSOR,
];

const DEVICE_EXTENSIONS: [*const i8; 1] =
    [ffi::VK_KHR_SWAPCHAIN_EXTENSION_NAME as *const u8 as *const i8];

fn check_validation_layer_support() -> bool {
    let mut layer_count: u32 = 0;
    unsafe {
        ffi::vkEnumerateInstanceLayerProperties(
            std::ptr::addr_of_mut!(layer_count),
            std::ptr::null_mut(),
        );
    }

    let mut layers: Vec<ffi::VkLayerProperties> = Vec::with_capacity(layer_count as usize);
    layers.resize(layer_count as usize, unsafe { std::mem::zeroed() });

    unsafe {
        ffi::vkEnumerateInstanceLayerProperties(
            std::ptr::addr_of_mut!(layer_count),
            layers.as_mut_ptr(),
        );
    }

    for layer_name in VALIDATION_LAYERS {
        let mut layer_found = false;
        let ln_cstr = unsafe { CStr::from_ptr(layer_name as *const i8) };
        for layer_prop in &layers {
            let lp_cstr: &CStr = unsafe { CStr::from_ptr(layer_prop.layerName.as_ptr()) };
            if ln_cstr == lp_cstr {
                layer_found = true;
                break;
            }
        }

        if !layer_found {
            return false;
        }
    }

    println!("Validation layers available");
    true
}

extern "C" fn validation_debug_callback(
    _message_severity: ffi::VkDebugUtilsMessageSeverityFlagBitsEXT,
    _message_type: ffi::VkDebugUtilsMessageTypeFlagsEXT,
    callback_data: *const ffi::VkDebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void,
) -> u32 {
    let message: &CStr = unsafe { CStr::from_ptr((*callback_data).pMessage) };

    println!(
        "validation layer: {}",
        message.to_str().unwrap_or("INVALID UTF-8 STRING")
    );

    ffi::VK_FALSE
}

fn create_debug_utils_messenger_ext(
    instance: ffi::VkInstance,
    create_info: *const ffi::VkDebugUtilsMessengerCreateInfoEXT,
    allocator: *const ffi::VkAllocationCallbacks,
    debug_messenger: *mut ffi::VkDebugUtilsMessengerEXT,
) -> i32 {
    let func_opt: ffi::PFN_vkCreateDebugUtilsMessengerEXT = unsafe {
        std::mem::transmute(ffi::vkGetInstanceProcAddr(
            instance,
            "vkCreateDebugUtilsMessengerEXT\x00".as_ptr() as *const i8,
        ))
    };

    if let Some(func) = func_opt {
        unsafe { func(instance, create_info, allocator, debug_messenger) }
    } else {
        ffi::VkResult_VK_ERROR_EXTENSION_NOT_PRESENT
    }
}

fn create_debug_messenger_create_info() -> ffi::VkDebugUtilsMessengerCreateInfoEXT {
    ffi::VkDebugUtilsMessengerCreateInfoEXT {
        sType: ffi::VkStructureType_VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        pNext: std::ptr::null(),
        flags: 0,
        messageSeverity: ffi::VkDebugUtilsMessageSeverityFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_SEVERITY_VERBOSE_BIT_EXT | ffi::VkDebugUtilsMessageSeverityFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT | ffi::VkDebugUtilsMessageSeverityFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT,
        messageType: ffi::VkDebugUtilsMessageTypeFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT | ffi::VkDebugUtilsMessageTypeFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT | ffi::VkDebugUtilsMessageTypeFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_TYPE_PERFORMANCE_BIT_EXT,
        pfnUserCallback: Some(validation_debug_callback),
        pUserData: std::ptr::null_mut(),
    }
}

struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>,
}

impl QueueFamilyIndices {
    fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

struct SwapChainSupportDetails {
    capabilities: ffi::VkSurfaceCapabilitiesKHR,
    formats: Vec<ffi::VkSurfaceFormatKHR>,
    present_modes: Vec<ffi::VkPresentModeKHR>,
}

impl Default for SwapChainSupportDetails {
    fn default() -> Self {
        Self {
            capabilities: unsafe { std::mem::zeroed() },
            formats: Vec::new(),
            present_modes: Vec::new(),
        }
    }
}

struct ShaderModuleWrapper {
    module: ffi::VkShaderModule,
    device: ffi::VkDevice,
}

impl ShaderModuleWrapper {
    pub fn get_module(&self) -> ffi::VkShaderModule {
        self.module
    }
}

impl Drop for ShaderModuleWrapper {
    fn drop(&mut self) {
        if !self.module.is_null() && !self.device.is_null() {
            unsafe {
                ffi::vkDestroyShaderModule(self.device, self.module, std::ptr::null());
            }
        }
    }
}

struct VulkanApp {
    window: *mut ffi::GLFWwindow,
    vk_instance: ffi::VkInstance,
    debug_messenger: ffi::VkDebugUtilsMessengerEXT,
    surface: ffi::VkSurfaceKHR,
    physical_device: ffi::VkPhysicalDevice,
    device: ffi::VkDevice,
    graphics_queue: ffi::VkQueue,
    present_queue: ffi::VkQueue,
    swap_chain: ffi::VkSwapchainKHR,
    swap_chain_images: Vec<ffi::VkImage>,
    swap_chain_image_format: ffi::VkFormat,
    swap_chain_extent: ffi::VkExtent2D,
    swap_chain_image_views: Vec<ffi::VkImageView>,
    render_pass: ffi::VkRenderPass,
    pipeline_layout: ffi::VkPipelineLayout,
    graphics_pipeline: ffi::VkPipeline,
    swap_chain_framebuffers: Vec<ffi::VkFramebuffer>,
    command_pool: ffi::VkCommandPool,
    command_buffer: ffi::VkCommandBuffer,
}

impl VulkanApp {
    fn new() -> Self {
        Self {
            window: std::ptr::null_mut(),
            vk_instance: std::ptr::null_mut(),
            debug_messenger: std::ptr::null_mut(),
            surface: std::ptr::null_mut(),
            physical_device: std::ptr::null_mut(),
            device: std::ptr::null_mut(),
            graphics_queue: std::ptr::null_mut(),
            present_queue: std::ptr::null_mut(),
            swap_chain: std::ptr::null_mut(),
            swap_chain_images: Vec::new(),
            swap_chain_image_format: 0,
            swap_chain_extent: unsafe { std::mem::zeroed() },
            swap_chain_image_views: Vec::new(),
            render_pass: std::ptr::null_mut(),
            pipeline_layout: std::ptr::null_mut(),
            graphics_pipeline: std::ptr::null_mut(),
            swap_chain_framebuffers: Vec::new(),
            command_pool: std::ptr::null_mut(),
            command_buffer: std::ptr::null_mut(),
        }
    }

    fn init_glfw(&mut self) {
        let app_title = CString::new("Vulkan").unwrap();
        unsafe {
            ffi::glfwInit();
            ffi::glfwWindowHint(ffi::GLFW_CLIENT_API as i32, ffi::GLFW_NO_API as i32);
            ffi::glfwWindowHint(ffi::GLFW_RESIZABLE as i32, ffi::GLFW_FALSE as i32);
            self.window = ffi::glfwCreateWindow(
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                app_title.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if self.window.is_null() {
                panic!("ERROR: Failed to create glfw window!");
            }
        }
    }

    fn init_vulkan(&mut self) {
        // Check validation layers before creating instance.
        if ENABLE_VALIDATION_LAYERS && !check_validation_layer_support() {
            panic!("Validation layers requested, but not available!");
        }

        self.create_instance().unwrap();
        self.setup_debug_messenger().unwrap();
        self.create_surface().unwrap();
        self.pick_physical_device().unwrap();
        self.create_logical_device().unwrap();
        self.create_swap_chain().unwrap();
        self.create_image_views().unwrap();
        self.create_render_pass().unwrap();
        self.create_graphics_pipeline()
            .expect("Should be able to set up graphics pipeline");
        self.create_framebuffers().unwrap();
        self.create_command_pool().unwrap();
        self.create_command_buffer().unwrap();
    }

    fn create_instance(&mut self) -> Result<(), String> {
        let app_name = CString::new("Vulkan Triangle").unwrap();
        let engine_name = CString::new("No Engine").unwrap();
        let app_info = ffi::VkApplicationInfo {
            sType: ffi::VkStructureType_VK_STRUCTURE_TYPE_APPLICATION_INFO,
            pNext: std::ptr::null_mut(),
            pApplicationName: app_name.as_ptr(),
            applicationVersion: ffi::VK_MAKE_VERSION(1, 0, 0),
            pEngineName: engine_name.as_ptr(),
            engineVersion: ffi::VK_MAKE_VERSION(1, 0, 0),
            apiVersion: ffi::VK_MAKE_API_VERSION(0, 1, 0, 0),
        };

        // Populate VkInstanceCreateInfo.

        // First get info from glfw.
        let mut ext_count: u32 = 0;
        let exts: *mut *const std::ffi::c_char;
        unsafe {
            exts = ffi::glfwGetRequiredInstanceExtensions(std::ptr::addr_of_mut!(ext_count));
        }

        let mut exts_with_validation: Vec<*const std::ffi::c_char> =
            Vec::with_capacity(ext_count as usize + 1);
        let validation_string: *const std::ffi::c_char =
            ffi::VK_EXT_DEBUG_UTILS_EXTENSION_NAME.as_ptr() as *const i8;
        if ENABLE_VALIDATION_LAYERS {
            let exts_slice: &[*const std::ffi::c_char] =
                unsafe { std::slice::from_raw_parts(exts, ext_count as usize) };
            for i in 0..(ext_count as usize) {
                exts_with_validation.push(exts_slice[i]);
            }
            exts_with_validation.push(validation_string);
        }

        // Second populate the struct with necessary info.
        let mut create_info = ffi::VkInstanceCreateInfo {
            sType: ffi::VkStructureType_VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            pApplicationInfo: std::ptr::addr_of!(app_info),
            enabledLayerCount: 0,
            ppEnabledLayerNames: std::ptr::null(),
            enabledExtensionCount: if ENABLE_VALIDATION_LAYERS {
                ext_count + 1
            } else {
                ext_count
            },
            ppEnabledExtensionNames: if ENABLE_VALIDATION_LAYERS {
                exts_with_validation.as_ptr()
            } else {
                exts
            },
        };

        let debug_messenger_create_info = create_debug_messenger_create_info();
        if ENABLE_VALIDATION_LAYERS {
            create_info.enabledLayerCount = VALIDATION_LAYERS.len() as u32;
            create_info.ppEnabledLayerNames = VALIDATION_LAYERS.as_ptr() as *const *const i8;

            create_info.pNext =
                std::ptr::addr_of!(debug_messenger_create_info) as *const std::ffi::c_void;
        }

        let vk_result = unsafe {
            ffi::vkCreateInstance(
                std::ptr::addr_of!(create_info),
                std::ptr::null(),
                std::ptr::addr_of_mut!(self.vk_instance),
            )
        };

        if vk_result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to create Vulkan instance"));
        }

        Ok(())
    }

    fn setup_debug_messenger(&mut self) -> Result<(), String> {
        if !ENABLE_VALIDATION_LAYERS {
            return Ok(());
        }

        if self.vk_instance.is_null() {
            return Err(String::from(
                "Cannot set up debug messenger if vk_instance is not initialized!",
            ));
        }

        let create_info = create_debug_messenger_create_info();

        let result = create_debug_utils_messenger_ext(
            self.vk_instance,
            std::ptr::addr_of!(create_info),
            std::ptr::null(),
            std::ptr::addr_of_mut!(self.debug_messenger),
        );
        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to set up debug messenger!"));
        }

        Ok(())
    }

    fn pick_physical_device(&mut self) -> Result<(), String> {
        let mut dev_count: u32 = 0;
        unsafe {
            ffi::vkEnumeratePhysicalDevices(
                self.vk_instance,
                std::ptr::addr_of_mut!(dev_count),
                std::ptr::null_mut(),
            );
        }

        if dev_count == 0 {
            return Err(String::from("Failed to find GPUs with Vulkan support!"));
        }

        let mut phys_dev_handles_vec: Vec<ffi::VkPhysicalDevice> =
            Vec::with_capacity(dev_count as usize);
        phys_dev_handles_vec.resize(dev_count as usize, std::ptr::null_mut());
        unsafe {
            ffi::vkEnumeratePhysicalDevices(
                self.vk_instance,
                std::ptr::addr_of_mut!(dev_count),
                phys_dev_handles_vec.as_mut_ptr(),
            );
        }

        for phys_dev in phys_dev_handles_vec {
            if self.is_device_suitable(phys_dev)? {
                self.physical_device = phys_dev;
                break;
            }
        }

        if self.physical_device.is_null() {
            return Err(String::from("Failed to find a suitable GPU!"));
        }

        Ok(())
    }

    fn create_logical_device(&mut self) -> Result<(), String> {
        if self.physical_device.is_null() {
            return Err(String::from(
                "\"physical_device\" must be set before calling \"create_logical_device\"!",
            ));
        }

        let indices = self.find_queue_families(self.physical_device);

        let mut dev_queue_create_infos: Vec<ffi::VkDeviceQueueCreateInfo> = Vec::new();
        let mut unique_queue_families: HashSet<u32> = HashSet::new();
        unique_queue_families.insert(indices.graphics_family.unwrap());
        unique_queue_families.insert(indices.present_family.unwrap());

        let queue_priority: f32 = 1.0;

        for queue_family in unique_queue_families {
            let mut dev_queue_create_info: ffi::VkDeviceQueueCreateInfo =
                unsafe { std::mem::zeroed() };
            dev_queue_create_info.sType =
                ffi::VkStructureType_VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
            dev_queue_create_info.queueFamilyIndex = queue_family;
            dev_queue_create_info.queueCount = 1;
            dev_queue_create_info.pQueuePriorities = std::ptr::addr_of!(queue_priority);
            dev_queue_create_infos.push(dev_queue_create_info);
        }

        let phys_dev_feat: ffi::VkPhysicalDeviceFeatures = unsafe { std::mem::zeroed() };

        let mut dev_create_info: ffi::VkDeviceCreateInfo = unsafe { std::mem::zeroed() };
        dev_create_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
        dev_create_info.pQueueCreateInfos = dev_queue_create_infos.as_ptr();
        dev_create_info.queueCreateInfoCount = dev_queue_create_infos.len() as u32;
        dev_create_info.pEnabledFeatures = std::ptr::addr_of!(phys_dev_feat);

        dev_create_info.ppEnabledExtensionNames = DEVICE_EXTENSIONS.as_ptr();
        dev_create_info.enabledExtensionCount = DEVICE_EXTENSIONS.len() as u32;

        if ENABLE_VALIDATION_LAYERS {
            dev_create_info.enabledLayerCount = VALIDATION_LAYERS.len() as u32;
            dev_create_info.ppEnabledLayerNames = VALIDATION_LAYERS.as_ptr() as *const *const i8;
        } else {
            dev_create_info.enabledLayerCount = 0;
        }

        let result = unsafe {
            ffi::vkCreateDevice(
                self.physical_device,
                std::ptr::addr_of!(dev_create_info),
                std::ptr::null(),
                std::ptr::addr_of_mut!(self.device),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to create logical device!"));
        }

        unsafe {
            ffi::vkGetDeviceQueue(
                self.device,
                indices.graphics_family.unwrap(),
                0,
                std::ptr::addr_of_mut!(self.graphics_queue),
            );
            ffi::vkGetDeviceQueue(
                self.device,
                indices.present_family.unwrap(),
                0,
                std::ptr::addr_of_mut!(self.present_queue),
            );
        }

        Ok(())
    }

    fn create_surface(&mut self) -> Result<(), String> {
        let result = unsafe {
            ffi::glfwCreateWindowSurface(
                self.vk_instance,
                self.window,
                std::ptr::null(),
                std::ptr::addr_of_mut!(self.surface),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to create window surface!"));
        }

        Ok(())
    }

    fn main_loop(&mut self) {
        if self.window.is_null() {
            panic!("ERROR: Cannot execute main loop if window is null!");
        }

        if self.vk_instance.is_null() {
            panic!("ERROR: Cannot execute main loop if vk_instance is null!");
        }

        loop {
            unsafe {
                if ffi::glfwWindowShouldClose(self.window) != 0 {
                    return;
                }
                ffi::glfwPollEvents();
            }
        }
    }

    fn find_queue_families(&self, dev: ffi::VkPhysicalDevice) -> QueueFamilyIndices {
        let mut queue_fam = QueueFamilyIndices {
            graphics_family: None,
            present_family: None,
        };

        let mut queue_family_count: u32 = 0;
        unsafe {
            ffi::vkGetPhysicalDeviceQueueFamilyProperties(
                dev,
                std::ptr::addr_of_mut!(queue_family_count),
                std::ptr::null_mut(),
            );
        }

        let mut queue_family_props: Vec<ffi::VkQueueFamilyProperties> =
            Vec::with_capacity(queue_family_count as usize);
        queue_family_props.resize(queue_family_count as usize, unsafe { std::mem::zeroed() });
        unsafe {
            ffi::vkGetPhysicalDeviceQueueFamilyProperties(
                dev,
                std::ptr::addr_of_mut!(queue_family_count),
                queue_family_props.as_mut_ptr(),
            );
        }

        for (idx, queue_family_prop) in queue_family_props.iter().enumerate() {
            let mut present_support: ffi::VkBool32 = ffi::VK_FALSE;
            unsafe {
                ffi::vkGetPhysicalDeviceSurfaceSupportKHR(
                    dev,
                    idx as u32,
                    self.surface,
                    std::ptr::addr_of_mut!(present_support),
                );
            }
            if present_support != ffi::VK_FALSE {
                queue_fam.present_family = Some(idx as u32);
            }
            if queue_family_prop.queueFlags & ffi::VkQueueFlagBits_VK_QUEUE_GRAPHICS_BIT != 0 {
                queue_fam.graphics_family = Some(idx as u32);
            }

            if queue_fam.is_complete() {
                break;
            }
        }

        queue_fam
    }

    fn is_device_suitable(&self, dev: ffi::VkPhysicalDevice) -> Result<bool, String> {
        let mut dev_props: ffi::VkPhysicalDeviceProperties = unsafe { std::mem::zeroed() };
        unsafe {
            ffi::vkGetPhysicalDeviceProperties(dev, std::ptr::addr_of_mut!(dev_props));
        }

        let mut dev_feat: ffi::VkPhysicalDeviceFeatures = unsafe { std::mem::zeroed() };
        unsafe {
            ffi::vkGetPhysicalDeviceFeatures(dev, std::ptr::addr_of_mut!(dev_feat));
        }

        // dev_props.deviceType == ffi::VkPhysicalDeviceType_VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU
        // && dev_feat.geometryShader != 0

        // Use previous checks for specifics, but for now, accept GPUs with required support.
        let extensions_supported = self.check_device_extensions_support(dev);

        let mut swap_chain_adequate = false;
        if extensions_supported {
            let swap_chain_support = self.query_swap_chain_support(dev)?;
            swap_chain_adequate = !swap_chain_support.formats.is_empty()
                && !swap_chain_support.present_modes.is_empty();
        }

        Ok(self.find_queue_families(dev).is_complete()
            && extensions_supported
            && swap_chain_adequate)
    }

    fn check_device_extensions_support(&self, dev: ffi::VkPhysicalDevice) -> bool {
        let mut req_extensions: HashSet<CString> = HashSet::new();
        for dev_ext in DEVICE_EXTENSIONS {
            let cstr = unsafe { CStr::from_ptr(dev_ext) };
            req_extensions.insert(cstr.to_owned());
        }

        let mut extension_count: u32 = 0;
        unsafe {
            ffi::vkEnumerateDeviceExtensionProperties(
                dev,
                std::ptr::null(),
                std::ptr::addr_of_mut!(extension_count),
                std::ptr::null_mut(),
            );
        }

        let mut available_extensions: Vec<ffi::VkExtensionProperties> =
            Vec::with_capacity(extension_count as usize);
        available_extensions.resize(extension_count as usize, unsafe { std::mem::zeroed() });
        unsafe {
            ffi::vkEnumerateDeviceExtensionProperties(
                dev,
                std::ptr::null(),
                std::ptr::addr_of_mut!(extension_count),
                available_extensions.as_mut_ptr(),
            );
        }

        for available in available_extensions {
            let cstr = unsafe { CStr::from_ptr(&available.extensionName as *const i8) };
            let cstring = cstr.to_owned();
            req_extensions.remove(&cstring);
        }

        req_extensions.is_empty()
    }

    fn query_swap_chain_support(
        &self,
        device: ffi::VkPhysicalDevice,
    ) -> Result<SwapChainSupportDetails, String> {
        if self.surface.is_null() {
            return Err(String::from(
                "surface must be initialized before calling query_swap_chain_support!",
            ));
        }

        let mut swap_chain_support_details = SwapChainSupportDetails::default();

        unsafe {
            ffi::vkGetPhysicalDeviceSurfaceCapabilitiesKHR(
                device,
                self.surface,
                std::ptr::addr_of_mut!(swap_chain_support_details.capabilities),
            );
        }

        let mut format_count: u32 = 0;
        unsafe {
            ffi::vkGetPhysicalDeviceSurfaceFormatsKHR(
                device,
                self.surface,
                std::ptr::addr_of_mut!(format_count),
                std::ptr::null_mut(),
            );
        }
        if format_count != 0 {
            swap_chain_support_details
                .formats
                .resize(format_count as usize, unsafe { std::mem::zeroed() });
            unsafe {
                ffi::vkGetPhysicalDeviceSurfaceFormatsKHR(
                    device,
                    self.surface,
                    std::ptr::addr_of_mut!(format_count),
                    swap_chain_support_details.formats.as_mut_ptr(),
                );
            }
        }

        let mut present_mode_count: u32 = 0;
        unsafe {
            ffi::vkGetPhysicalDeviceSurfacePresentModesKHR(
                device,
                self.surface,
                std::ptr::addr_of_mut!(present_mode_count),
                std::ptr::null_mut(),
            );
        }
        if present_mode_count != 0 {
            swap_chain_support_details
                .present_modes
                .resize(present_mode_count as usize, unsafe { std::mem::zeroed() });
            unsafe {
                ffi::vkGetPhysicalDeviceSurfacePresentModesKHR(
                    device,
                    self.surface,
                    std::ptr::addr_of_mut!(present_mode_count),
                    swap_chain_support_details.present_modes.as_mut_ptr(),
                );
            }
        }

        Ok(swap_chain_support_details)
    }

    fn choose_swap_surface_format(
        &self,
        available_formats: &Vec<ffi::VkSurfaceFormatKHR>,
    ) -> Option<usize> {
        if available_formats.is_empty() {
            return None;
        }

        for (idx, format) in available_formats.iter().enumerate() {
            if format.format == ffi::VkFormat_VK_FORMAT_B8G8R8A8_SRGB
                && format.colorSpace == ffi::VkColorSpaceKHR_VK_COLOR_SPACE_SRGB_NONLINEAR_KHR
            {
                return Some(idx);
            }
        }

        Some(0)
    }

    fn choose_swap_present_mode(
        &self,
        available_present_modes: &Vec<ffi::VkPresentModeKHR>,
    ) -> ffi::VkPresentModeKHR {
        for mode in available_present_modes {
            if *mode == ffi::VkPresentModeKHR_VK_PRESENT_MODE_MAILBOX_KHR {
                return ffi::VkPresentModeKHR_VK_PRESENT_MODE_MAILBOX_KHR;
            }
        }

        ffi::VkPresentModeKHR_VK_PRESENT_MODE_FIFO_KHR
    }

    fn choose_swap_extent(&self, capabilities: &ffi::VkSurfaceCapabilitiesKHR) -> ffi::VkExtent2D {
        if capabilities.currentExtent.width != u32::MAX {
            return capabilities.currentExtent;
        }

        let mut width: i32 = 0;
        let mut height: i32 = 0;
        unsafe {
            ffi::glfwGetFramebufferSize(
                self.window,
                std::ptr::addr_of_mut!(width),
                std::ptr::addr_of_mut!(height),
            );
        }

        let mut actual_extent = ffi::VkExtent2D {
            width: width as u32,
            height: height as u32,
        };

        actual_extent.width = actual_extent.width.clamp(
            capabilities.minImageExtent.width,
            capabilities.maxImageExtent.width,
        );
        actual_extent.height = actual_extent.height.clamp(
            capabilities.minImageExtent.height,
            capabilities.maxImageExtent.height,
        );

        actual_extent
    }

    fn create_swap_chain(&mut self) -> Result<(), String> {
        let swap_chain_support = self.query_swap_chain_support(self.physical_device)?;

        let surface_format_idx = self
            .choose_swap_surface_format(&swap_chain_support.formats)
            .expect("surface format must exist");
        let present_mode = self.choose_swap_present_mode(&swap_chain_support.present_modes);
        let extent = self.choose_swap_extent(&swap_chain_support.capabilities);

        let mut image_count: u32 = swap_chain_support.capabilities.minImageCount + 1;
        if swap_chain_support.capabilities.maxImageCount > 0
            && image_count > swap_chain_support.capabilities.maxImageCount
        {
            image_count = swap_chain_support.capabilities.maxImageCount;
        }

        let mut create_info: ffi::VkSwapchainCreateInfoKHR = unsafe { std::mem::zeroed() };
        create_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
        create_info.surface = self.surface;

        create_info.minImageCount = image_count;
        create_info.imageFormat = swap_chain_support.formats[surface_format_idx].format;
        create_info.imageColorSpace = swap_chain_support.formats[surface_format_idx].colorSpace;
        create_info.imageExtent = extent;
        create_info.imageArrayLayers = 1;
        create_info.imageUsage = ffi::VkImageUsageFlagBits_VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT;

        let indices = self.find_queue_families(self.physical_device);
        let indices_arr: [u32; 2] = [
            indices.graphics_family.unwrap(),
            indices.present_family.unwrap(),
        ];

        if indices.graphics_family != indices.present_family {
            create_info.imageSharingMode = ffi::VkSharingMode_VK_SHARING_MODE_CONCURRENT;
            create_info.queueFamilyIndexCount = 2;
            create_info.pQueueFamilyIndices = indices_arr.as_ptr();
        } else {
            create_info.imageSharingMode = ffi::VkSharingMode_VK_SHARING_MODE_EXCLUSIVE;
            create_info.queueFamilyIndexCount = 0;
            create_info.pQueueFamilyIndices = std::ptr::null();
        }

        create_info.preTransform = swap_chain_support.capabilities.currentTransform;

        create_info.compositeAlpha =
            ffi::VkCompositeAlphaFlagBitsKHR_VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;

        create_info.presentMode = present_mode;
        create_info.clipped = ffi::VK_TRUE;

        create_info.oldSwapchain = std::ptr::null_mut();

        let result = unsafe {
            ffi::vkCreateSwapchainKHR(
                self.device,
                std::ptr::addr_of!(create_info),
                std::ptr::null(),
                std::ptr::addr_of_mut!(self.swap_chain),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to create swap chain!"));
        }

        unsafe {
            ffi::vkGetSwapchainImagesKHR(
                self.device,
                self.swap_chain,
                std::ptr::addr_of_mut!(image_count),
                std::ptr::null_mut(),
            );
            self.swap_chain_images
                .resize(image_count as usize, std::ptr::null_mut());
            ffi::vkGetSwapchainImagesKHR(
                self.device,
                self.swap_chain,
                std::ptr::addr_of_mut!(image_count),
                self.swap_chain_images.as_mut_ptr(),
            );
        }

        self.swap_chain_image_format = swap_chain_support.formats[surface_format_idx].format;
        self.swap_chain_extent = extent;

        Ok(())
    }

    fn create_image_views(&mut self) -> Result<(), String> {
        self.swap_chain_image_views
            .resize(self.swap_chain_images.len(), std::ptr::null_mut());

        for (idx, image) in self.swap_chain_images.iter().enumerate() {
            let mut create_info: ffi::VkImageViewCreateInfo = unsafe { std::mem::zeroed() };
            create_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
            create_info.image = *image;

            create_info.viewType = ffi::VkImageViewType_VK_IMAGE_VIEW_TYPE_2D;
            create_info.format = self.swap_chain_image_format;

            create_info.components.r = ffi::VkComponentSwizzle_VK_COMPONENT_SWIZZLE_IDENTITY;
            create_info.components.g = ffi::VkComponentSwizzle_VK_COMPONENT_SWIZZLE_IDENTITY;
            create_info.components.b = ffi::VkComponentSwizzle_VK_COMPONENT_SWIZZLE_IDENTITY;
            create_info.components.a = ffi::VkComponentSwizzle_VK_COMPONENT_SWIZZLE_IDENTITY;

            create_info.subresourceRange.aspectMask =
                ffi::VkImageAspectFlagBits_VK_IMAGE_ASPECT_COLOR_BIT;
            create_info.subresourceRange.baseMipLevel = 0;
            create_info.subresourceRange.levelCount = 1;
            create_info.subresourceRange.baseArrayLayer = 0;
            create_info.subresourceRange.layerCount = 1;

            let result = unsafe {
                ffi::vkCreateImageView(
                    self.device,
                    std::ptr::addr_of!(create_info),
                    std::ptr::null(),
                    std::ptr::addr_of_mut!(self.swap_chain_image_views[idx]),
                )
            };
            if result != ffi::VkResult_VK_SUCCESS {
                return Err(format!("Failed to create image view {}!", idx));
            }
        }

        Ok(())
    }

    fn create_graphics_pipeline(&mut self) -> Result<(), String> {
        let vert_shader_module = self.create_vertex_shader_module()?;
        let frag_shader_module = self.create_fragment_shader_module()?;

        let mut vert_shader_stage_info: ffi::VkPipelineShaderStageCreateInfo =
            unsafe { std::mem::zeroed() };
        vert_shader_stage_info.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
        vert_shader_stage_info.stage = ffi::VkShaderStageFlagBits_VK_SHADER_STAGE_VERTEX_BIT;
        vert_shader_stage_info.module = vert_shader_module.get_module();
        vert_shader_stage_info.pName = "main\x00".as_ptr() as *const i8;

        let mut frag_shader_stage_info: ffi::VkPipelineShaderStageCreateInfo =
            unsafe { std::mem::zeroed() };
        frag_shader_stage_info.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
        frag_shader_stage_info.stage = ffi::VkShaderStageFlagBits_VK_SHADER_STAGE_FRAGMENT_BIT;
        frag_shader_stage_info.module = frag_shader_module.get_module();
        frag_shader_stage_info.pName = "main\x00".as_ptr() as *const i8;

        let shader_stages: [ffi::VkPipelineShaderStageCreateInfo; 2] =
            [vert_shader_stage_info, frag_shader_stage_info];

        let vertex_input_info = Self::create_vertex_input_state_info_struct();

        let mut input_assembly: ffi::VkPipelineInputAssemblyStateCreateInfo =
            unsafe { std::mem::zeroed() };
        input_assembly.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO;
        input_assembly.topology = ffi::VkPrimitiveTopology_VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST;
        input_assembly.primitiveRestartEnable = ffi::VK_FALSE;

        let dynamic_state_info_struct = Self::create_dynamic_state_info_struct();

        let viewport_state = Self::create_viewport_state_info_struct();

        let rasterizer_info = Self::create_rasterizer_info_struct();

        let multisampling_info = Self::create_multisampling_info_struct();

        let color_blend_attachment = Self::create_color_blend_attach_state_struct();

        let color_blend_info_struct =
            Self::create_color_blend_state_info_struct(std::ptr::addr_of!(color_blend_attachment));

        let mut pipeline_layout_info: ffi::VkPipelineLayoutCreateInfo =
            unsafe { std::mem::zeroed() };
        pipeline_layout_info.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
        pipeline_layout_info.setLayoutCount = 0;
        pipeline_layout_info.pSetLayouts = std::ptr::null();
        pipeline_layout_info.pushConstantRangeCount = 0;
        pipeline_layout_info.pPushConstantRanges = std::ptr::null();

        let result = unsafe {
            ffi::vkCreatePipelineLayout(
                self.device,
                std::ptr::addr_of!(pipeline_layout_info),
                std::ptr::null(),
                std::ptr::addr_of_mut!(self.pipeline_layout),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to create pipeline layout!"));
        }

        let mut pipeline_info: ffi::VkGraphicsPipelineCreateInfo = unsafe { std::mem::zeroed() };
        pipeline_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
        pipeline_info.stageCount = 2;
        pipeline_info.pStages = shader_stages.as_ptr();

        pipeline_info.pVertexInputState = std::ptr::addr_of!(vertex_input_info);
        pipeline_info.pInputAssemblyState = std::ptr::addr_of!(input_assembly);
        pipeline_info.pViewportState = std::ptr::addr_of!(viewport_state);
        pipeline_info.pRasterizationState = std::ptr::addr_of!(rasterizer_info);
        pipeline_info.pMultisampleState = std::ptr::addr_of!(multisampling_info);
        pipeline_info.pDepthStencilState = std::ptr::null();
        pipeline_info.pColorBlendState = std::ptr::addr_of!(color_blend_info_struct);
        pipeline_info.pDynamicState = std::ptr::addr_of!(dynamic_state_info_struct);

        pipeline_info.layout = self.pipeline_layout;

        pipeline_info.renderPass = self.render_pass;
        pipeline_info.subpass = 0;

        pipeline_info.basePipelineHandle = std::ptr::null_mut();
        pipeline_info.basePipelineIndex = -1;

        let result = unsafe {
            ffi::vkCreateGraphicsPipelines(
                self.device,
                std::ptr::null_mut(),
                1,
                std::ptr::addr_of!(pipeline_info),
                std::ptr::null(),
                std::ptr::addr_of_mut!(self.graphics_pipeline),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to create a graphics pipeline!"));
        }

        // TODO: Use the *_shader_stage_info structs before vert/frag_shader_module is cleaned up.
        Ok(())
    }

    fn create_vertex_shader_module(&mut self) -> Result<ShaderModuleWrapper, String> {
        let vertex_shader = std::include_bytes!(concat!(env!("OUT_DIR"), "/vert.spv"));

        let mut create_info: ffi::VkShaderModuleCreateInfo = unsafe { std::mem::zeroed() };
        create_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
        create_info.codeSize = vertex_shader.len();
        create_info.pCode = vertex_shader.as_ptr() as *const u32;

        let mut shader_module: ffi::VkShaderModule = unsafe { std::mem::zeroed() };
        let result = unsafe {
            ffi::vkCreateShaderModule(
                self.device,
                std::ptr::addr_of!(create_info),
                std::ptr::null(),
                std::ptr::addr_of_mut!(shader_module),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            Err(String::from("Failed to create vertex shader module!"))
        } else {
            Ok(ShaderModuleWrapper {
                module: shader_module,
                device: self.device,
            })
        }
    }

    fn create_fragment_shader_module(&mut self) -> Result<ShaderModuleWrapper, String> {
        let fragment_shader = std::include_bytes!(concat!(env!("OUT_DIR"), "/frag.spv"));

        let mut create_info: ffi::VkShaderModuleCreateInfo = unsafe { std::mem::zeroed() };
        create_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
        create_info.codeSize = fragment_shader.len();
        create_info.pCode = fragment_shader.as_ptr() as *const u32;

        let mut shader_module: ffi::VkShaderModule = unsafe { std::mem::zeroed() };
        let result = unsafe {
            ffi::vkCreateShaderModule(
                self.device,
                std::ptr::addr_of!(create_info),
                std::ptr::null(),
                std::ptr::addr_of_mut!(shader_module),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            Err(String::from("Failed to create fragment shader module!"))
        } else {
            Ok(ShaderModuleWrapper {
                module: shader_module,
                device: self.device,
            })
        }
    }

    fn create_dynamic_state_info_struct() -> ffi::VkPipelineDynamicStateCreateInfo {
        let mut dynamic_state: ffi::VkPipelineDynamicStateCreateInfo =
            unsafe { std::mem::zeroed() };
        dynamic_state.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO;
        dynamic_state.dynamicStateCount = DYNAMIC_STATES.len() as u32;
        dynamic_state.pDynamicStates = DYNAMIC_STATES.as_ptr();

        dynamic_state
    }

    fn create_vertex_input_state_info_struct() -> ffi::VkPipelineVertexInputStateCreateInfo {
        let mut vertex_input_info: ffi::VkPipelineVertexInputStateCreateInfo =
            unsafe { std::mem::zeroed() };
        vertex_input_info.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO;
        vertex_input_info.vertexBindingDescriptionCount = 0;
        vertex_input_info.pVertexBindingDescriptions = std::ptr::null();
        vertex_input_info.vertexAttributeDescriptionCount = 0;
        vertex_input_info.pVertexBindingDescriptions = std::ptr::null();

        vertex_input_info
    }

    fn create_viewport(&self) -> ffi::VkViewport {
        let mut viewport: ffi::VkViewport = unsafe { std::mem::zeroed() };
        viewport.x = 0.0;
        viewport.y = 0.0;
        viewport.width = self.swap_chain_extent.width as f32;
        viewport.height = self.swap_chain_extent.height as f32;
        viewport.minDepth = 0.0;
        viewport.maxDepth = 1.0;

        viewport
    }

    fn create_scissor(&self) -> ffi::VkRect2D {
        ffi::VkRect2D {
            offset: ffi::VkOffset2D { x: 0, y: 0 },
            extent: self.swap_chain_extent,
        }
    }

    fn create_viewport_state_info_struct() -> ffi::VkPipelineViewportStateCreateInfo {
        let mut viewport_state: ffi::VkPipelineViewportStateCreateInfo =
            unsafe { std::mem::zeroed() };
        viewport_state.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO;
        viewport_state.viewportCount = 1;
        viewport_state.scissorCount = 1;

        viewport_state
    }

    fn create_rasterizer_info_struct() -> ffi::VkPipelineRasterizationStateCreateInfo {
        let mut rasterizer_info: ffi::VkPipelineRasterizationStateCreateInfo =
            unsafe { std::mem::zeroed() };
        rasterizer_info.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO;
        rasterizer_info.depthClampEnable = ffi::VK_FALSE;
        rasterizer_info.rasterizerDiscardEnable = ffi::VK_FALSE;
        rasterizer_info.polygonMode = ffi::VkPolygonMode_VK_POLYGON_MODE_FILL;
        rasterizer_info.lineWidth = 1.0;
        rasterizer_info.cullMode = ffi::VkCullModeFlagBits_VK_CULL_MODE_BACK_BIT;
        rasterizer_info.frontFace = ffi::VkFrontFace_VK_FRONT_FACE_CLOCKWISE;
        rasterizer_info.depthBiasEnable = ffi::VK_FALSE;
        rasterizer_info.depthBiasConstantFactor = 0.0;
        rasterizer_info.depthBiasClamp = 0.0;
        rasterizer_info.depthBiasSlopeFactor = 0.0;

        rasterizer_info
    }

    fn create_multisampling_info_struct() -> ffi::VkPipelineMultisampleStateCreateInfo {
        let mut multisampling_info: ffi::VkPipelineMultisampleStateCreateInfo =
            unsafe { std::mem::zeroed() };
        multisampling_info.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO;
        multisampling_info.sampleShadingEnable = ffi::VK_FALSE;
        multisampling_info.rasterizationSamples = ffi::VkSampleCountFlagBits_VK_SAMPLE_COUNT_1_BIT;
        multisampling_info.minSampleShading = 1.0;
        multisampling_info.pSampleMask = std::ptr::null();
        multisampling_info.alphaToCoverageEnable = ffi::VK_FALSE;
        multisampling_info.alphaToOneEnable = ffi::VK_FALSE;

        multisampling_info
    }

    fn create_color_blend_attach_state_struct() -> ffi::VkPipelineColorBlendAttachmentState {
        let mut color_blend_attachment: ffi::VkPipelineColorBlendAttachmentState =
            unsafe { std::mem::zeroed() };
        color_blend_attachment.colorWriteMask =
            ffi::VkColorComponentFlagBits_VK_COLOR_COMPONENT_R_BIT
                | ffi::VkColorComponentFlagBits_VK_COLOR_COMPONENT_G_BIT
                | ffi::VkColorComponentFlagBits_VK_COLOR_COMPONENT_B_BIT
                | ffi::VkColorComponentFlagBits_VK_COLOR_COMPONENT_A_BIT;
        color_blend_attachment.blendEnable = ffi::VK_FALSE;
        color_blend_attachment.srcColorBlendFactor = ffi::VkBlendFactor_VK_BLEND_FACTOR_ONE;
        color_blend_attachment.dstColorBlendFactor = ffi::VkBlendFactor_VK_BLEND_FACTOR_ZERO;
        color_blend_attachment.colorBlendOp = ffi::VkBlendOp_VK_BLEND_OP_ADD;
        color_blend_attachment.srcAlphaBlendFactor = ffi::VkBlendFactor_VK_BLEND_FACTOR_ONE;
        color_blend_attachment.dstAlphaBlendFactor = ffi::VkBlendFactor_VK_BLEND_FACTOR_ZERO;
        color_blend_attachment.alphaBlendOp = ffi::VkBlendOp_VK_BLEND_OP_ADD;

        color_blend_attachment
    }

    fn create_color_blend_state_info_struct(
        color_blend_attach_ptr: *const ffi::VkPipelineColorBlendAttachmentState,
    ) -> ffi::VkPipelineColorBlendStateCreateInfo {
        let mut color_blending: ffi::VkPipelineColorBlendStateCreateInfo =
            unsafe { std::mem::zeroed() };
        color_blending.sType =
            ffi::VkStructureType_VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO;
        color_blending.logicOpEnable = ffi::VK_FALSE;
        color_blending.logicOp = ffi::VkLogicOp_VK_LOGIC_OP_COPY;
        color_blending.attachmentCount = 1;
        color_blending.pAttachments = color_blend_attach_ptr;
        color_blending.blendConstants[0] = 0.0;
        color_blending.blendConstants[1] = 0.0;
        color_blending.blendConstants[2] = 0.0;
        color_blending.blendConstants[3] = 0.0;

        color_blending
    }

    fn create_render_pass(&mut self) -> Result<(), String> {
        let mut color_attachment: ffi::VkAttachmentDescription = unsafe { std::mem::zeroed() };
        color_attachment.format = self.swap_chain_image_format;
        color_attachment.samples = ffi::VkSampleCountFlagBits_VK_SAMPLE_COUNT_1_BIT;

        color_attachment.loadOp = ffi::VkAttachmentLoadOp_VK_ATTACHMENT_LOAD_OP_CLEAR;
        color_attachment.storeOp = ffi::VkAttachmentStoreOp_VK_ATTACHMENT_STORE_OP_STORE;

        color_attachment.stencilLoadOp = ffi::VkAttachmentLoadOp_VK_ATTACHMENT_LOAD_OP_DONT_CARE;
        color_attachment.stencilStoreOp = ffi::VkAttachmentStoreOp_VK_ATTACHMENT_STORE_OP_DONT_CARE;

        color_attachment.initialLayout = ffi::VkImageLayout_VK_IMAGE_LAYOUT_UNDEFINED;
        color_attachment.finalLayout = ffi::VkImageLayout_VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;

        let mut color_attachment_ref: ffi::VkAttachmentReference = unsafe { std::mem::zeroed() };
        color_attachment_ref.attachment = 0;
        color_attachment_ref.layout = ffi::VkImageLayout_VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

        let mut subpass: ffi::VkSubpassDescription = unsafe { std::mem::zeroed() };
        subpass.pipelineBindPoint = ffi::VkPipelineBindPoint_VK_PIPELINE_BIND_POINT_GRAPHICS;

        subpass.colorAttachmentCount = 1;
        subpass.pColorAttachments = std::ptr::addr_of!(color_attachment_ref);

        let mut render_pass_info: ffi::VkRenderPassCreateInfo = unsafe { std::mem::zeroed() };
        render_pass_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO;
        render_pass_info.attachmentCount = 1;
        render_pass_info.pAttachments = std::ptr::addr_of!(color_attachment);
        render_pass_info.subpassCount = 1;
        render_pass_info.pSubpasses = std::ptr::addr_of!(subpass);

        let result = unsafe {
            ffi::vkCreateRenderPass(
                self.device,
                std::ptr::addr_of!(render_pass_info),
                std::ptr::null(),
                std::ptr::addr_of_mut!(self.render_pass),
            )
        };

        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to create render pass!"));
        }

        Ok(())
    }

    fn create_framebuffers(&mut self) -> Result<(), String> {
        self.swap_chain_framebuffers
            .resize(self.swap_chain_image_views.len(), std::ptr::null_mut());

        for (idx, image_view) in self.swap_chain_image_views.iter().enumerate() {
            let mut framebuffer_info: ffi::VkFramebufferCreateInfo = unsafe { std::mem::zeroed() };
            framebuffer_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO;
            framebuffer_info.renderPass = self.render_pass;
            framebuffer_info.attachmentCount = 1;
            framebuffer_info.pAttachments = image_view as *const ffi::VkImageView;
            framebuffer_info.width = self.swap_chain_extent.width;
            framebuffer_info.height = self.swap_chain_extent.height;
            framebuffer_info.layers = 1;

            let result = unsafe {
                ffi::vkCreateFramebuffer(
                    self.device,
                    std::ptr::addr_of!(framebuffer_info),
                    std::ptr::null(),
                    std::ptr::addr_of_mut!(self.swap_chain_framebuffers[idx]),
                )
            };

            if result != ffi::VkResult_VK_SUCCESS {
                return Err(String::from("Failed to create framebuffer!"));
            }
        }

        Ok(())
    }

    fn create_command_pool(&mut self) -> Result<(), String> {
        let indices = self.find_queue_families(self.physical_device);

        let mut pool_info: ffi::VkCommandPoolCreateInfo = unsafe { std::mem::zeroed() };
        pool_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
        pool_info.flags =
            ffi::VkCommandPoolCreateFlagBits_VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT;
        pool_info.queueFamilyIndex = indices
            .graphics_family
            .expect("indices should have graphics family idx");

        let result = unsafe {
            ffi::vkCreateCommandPool(
                self.device,
                std::ptr::addr_of!(pool_info),
                std::ptr::null(),
                std::ptr::addr_of_mut!(self.command_pool),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to create command pool!"));
        }

        Ok(())
    }

    fn create_command_buffer(&mut self) -> Result<(), String> {
        let mut alloc_info: ffi::VkCommandBufferAllocateInfo = unsafe { std::mem::zeroed() };
        alloc_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
        alloc_info.commandPool = self.command_pool;
        alloc_info.level = ffi::VkCommandBufferLevel_VK_COMMAND_BUFFER_LEVEL_PRIMARY;
        alloc_info.commandBufferCount = 1;

        let result = unsafe {
            ffi::vkAllocateCommandBuffers(
                self.device,
                std::ptr::addr_of!(alloc_info),
                std::ptr::addr_of_mut!(self.command_buffer),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to allocate command buffers!"));
        }

        Ok(())
    }

    fn record_command_buffer(
        &mut self,
        command_buffer: ffi::VkCommandBuffer,
        image_index: usize,
    ) -> Result<(), String> {
        let mut begin_info: ffi::VkCommandBufferBeginInfo = unsafe { std::mem::zeroed() };
        begin_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
        begin_info.flags = 0;
        begin_info.pInheritanceInfo = std::ptr::null();

        let result =
            unsafe { ffi::vkBeginCommandBuffer(command_buffer, std::ptr::addr_of!(begin_info)) };
        if result != ffi::VkResult_VK_SUCCESS {
            return Err(String::from("Failed to begin recording command buffer!"));
        }

        let mut render_pass_info: ffi::VkRenderPassBeginInfo = unsafe { std::mem::zeroed() };
        render_pass_info.sType = ffi::VkStructureType_VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO;
        render_pass_info.renderPass = self.render_pass;
        render_pass_info.framebuffer = self.swap_chain_framebuffers[image_index];

        render_pass_info.renderArea.offset.x = 0;
        render_pass_info.renderArea.offset.y = 0;
        render_pass_info.renderArea.extent = self.swap_chain_extent;

        let mut clear_color: ffi::VkClearValue = unsafe { std::mem::zeroed() };
        unsafe {
            clear_color.color.float32[0] = 0.0;
            clear_color.color.float32[1] = 0.0;
            clear_color.color.float32[2] = 0.0;
            clear_color.color.float32[3] = 1.0;
        }
        render_pass_info.clearValueCount = 1;
        render_pass_info.pClearValues = std::ptr::addr_of!(clear_color);

        unsafe {
            ffi::vkCmdBeginRenderPass(
                command_buffer,
                std::ptr::addr_of!(render_pass_info),
                ffi::VkSubpassContents_VK_SUBPASS_CONTENTS_INLINE,
            );
            ffi::vkCmdBindPipeline(
                command_buffer,
                ffi::VkPipelineBindPoint_VK_PIPELINE_BIND_POINT_GRAPHICS,
                self.graphics_pipeline,
            );
        }

        let viewport = self.create_viewport();

        unsafe {
            ffi::vkCmdSetViewport(command_buffer, 0, 1, std::ptr::addr_of!(viewport));
        }

        let scissor = self.create_scissor();

        unsafe {
            ffi::vkCmdSetScissor(command_buffer, 0, 1, std::ptr::addr_of!(scissor));
            ffi::vkCmdDraw(command_buffer, 3, 1, 0, 0);
            ffi::vkCmdEndRenderPass(command_buffer);

            if ffi::vkEndCommandBuffer(command_buffer) != ffi::VkResult_VK_SUCCESS {
                return Err(String::from("Failed to record command buffer!"));
            }
        }

        Ok(())
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        if !self.command_pool.is_null() {
            unsafe {
                ffi::vkDestroyCommandPool(self.device, self.command_pool, std::ptr::null());
            }
        }

        for framebuffer in &self.swap_chain_framebuffers {
            unsafe {
                ffi::vkDestroyFramebuffer(self.device, *framebuffer, std::ptr::null());
            }
        }

        if !self.graphics_pipeline.is_null() {
            unsafe {
                ffi::vkDestroyPipeline(self.device, self.graphics_pipeline, std::ptr::null());
            }
        }

        if !self.pipeline_layout.is_null() {
            unsafe {
                ffi::vkDestroyPipelineLayout(self.device, self.pipeline_layout, std::ptr::null());
            }
        }

        if !self.render_pass.is_null() {
            unsafe {
                ffi::vkDestroyRenderPass(self.device, self.render_pass, std::ptr::null());
            }
        }

        for view in &self.swap_chain_image_views {
            unsafe {
                ffi::vkDestroyImageView(self.device, *view, std::ptr::null());
            }
        }

        if !self.swap_chain.is_null() {
            unsafe {
                ffi::vkDestroySwapchainKHR(self.device, self.swap_chain, std::ptr::null());
            }
        }

        if !self.device.is_null() {
            unsafe {
                ffi::vkDestroyDevice(self.device, std::ptr::null());
            }
        }

        if ENABLE_VALIDATION_LAYERS && !self.debug_messenger.is_null() {
            let func_opt: ffi::PFN_vkDestroyDebugUtilsMessengerEXT = unsafe {
                std::mem::transmute(ffi::vkGetInstanceProcAddr(
                    self.vk_instance,
                    "vkDestroyDebugUtilsMessengerEXT\x00".as_ptr() as *const i8,
                ))
            };

            if let Some(func) = func_opt {
                unsafe {
                    func(self.vk_instance, self.debug_messenger, std::ptr::null());
                }
            } else {
                println!("WARNING: Failed to load fn to unload debug messenger!");
            }
        }

        if !self.surface.is_null() {
            unsafe {
                ffi::vkDestroySurfaceKHR(self.vk_instance, self.surface, std::ptr::null());
            }
        }

        if !self.vk_instance.is_null() {
            unsafe {
                ffi::vkDestroyInstance(self.vk_instance, std::ptr::null());
            }
        }

        if !self.window.is_null() {
            unsafe {
                ffi::glfwDestroyWindow(self.window);
            }
        }

        unsafe {
            ffi::glfwTerminate();
        }
    }
}

fn main() {
    let mut app = VulkanApp::new();

    app.init_glfw();
    app.init_vulkan();
    app.main_loop();
}
