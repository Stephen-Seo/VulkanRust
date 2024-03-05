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

        self.create_instance();
        self.setup_debug_messenger();
        self.create_surface();
        self.pick_physical_device();
        self.create_logical_device();
        self.create_swap_chain();
    }

    fn create_instance(&mut self) {
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
            panic!("ERROR: Failed to create vk instance!");
        }
    }

    fn setup_debug_messenger(&mut self) {
        if !ENABLE_VALIDATION_LAYERS {
            return;
        }

        if self.vk_instance.is_null() {
            panic!("ERROR: Cannot set up debug messenger if vk_instance is not initialized!");
        }

        let create_info = create_debug_messenger_create_info();

        let result = create_debug_utils_messenger_ext(
            self.vk_instance,
            std::ptr::addr_of!(create_info),
            std::ptr::null(),
            std::ptr::addr_of_mut!(self.debug_messenger),
        );
        if result != ffi::VkResult_VK_SUCCESS {
            panic!("Failed to set up debug messenger!");
        }
    }

    fn pick_physical_device(&mut self) {
        let mut dev_count: u32 = 0;
        unsafe {
            ffi::vkEnumeratePhysicalDevices(
                self.vk_instance,
                std::ptr::addr_of_mut!(dev_count),
                std::ptr::null_mut(),
            );
        }

        if dev_count == 0 {
            panic!("Failed to find GPUs with Vulkan support!");
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
            if self.is_device_suitable(phys_dev) {
                self.physical_device = phys_dev;
                break;
            }
        }

        if self.physical_device.is_null() {
            panic!("Failed to find a suitable GPU!");
        }
    }

    fn create_logical_device(&mut self) {
        if self.physical_device.is_null() {
            panic!("\"physical_device\" must be set before calling \"create_logical_device\"!");
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

        let mut phys_dev_feat: ffi::VkPhysicalDeviceFeatures = unsafe { std::mem::zeroed() };

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
            panic!("Failed to create logical device!");
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
    }

    fn create_surface(&mut self) {
        let result = unsafe {
            ffi::glfwCreateWindowSurface(
                self.vk_instance,
                self.window,
                std::ptr::null(),
                std::ptr::addr_of_mut!(self.surface),
            )
        };
        if result != ffi::VkResult_VK_SUCCESS {
            panic!("Failed to create window surface!");
        }
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

    fn is_device_suitable(&self, dev: ffi::VkPhysicalDevice) -> bool {
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
            let swap_chain_support = self.query_swap_chain_support(dev);
            swap_chain_adequate = !swap_chain_support.formats.is_empty()
                && !swap_chain_support.present_modes.is_empty();
        }

        self.find_queue_families(dev).is_complete() && extensions_supported && swap_chain_adequate
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

    fn query_swap_chain_support(&self, device: ffi::VkPhysicalDevice) -> SwapChainSupportDetails {
        if self.surface.is_null() {
            panic!("surface must be initialized before calling query_swap_chain_support!");
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

        swap_chain_support_details
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

        return Some(0);
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
            return capabilities.currentExtent.clone();
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

    fn create_swap_chain(&mut self) {
        let swap_chain_support = self.query_swap_chain_support(self.physical_device);

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
            panic!("Failed to create swap chain!");
        }
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
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
