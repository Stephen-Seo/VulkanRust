mod ffi {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused_imports)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/glfw_vk_bindings.rs"));

    pub fn VK_MAKE_VERSION(major: u32, minor: u32, patch: u32) -> u32 {
        (major << 22) | (minor << 12) | patch
    }

    pub fn VK_MAKE_API_VERSION(variant: u32, major: u32, minor: u32, patch: u32) -> u32 {
        (variant << 29) | (major << 22) | (minor << 12) | patch
    }
}

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

fn check_validation_layer_support() -> bool {
    let mut layer_count: u32 = 0;
    unsafe {
        ffi::vkEnumerateInstanceLayerProperties(
            std::ptr::addr_of_mut!(layer_count),
            std::ptr::null_mut(),
        );
    }

    let mut layers: Vec<ffi::VkLayerProperties> = Vec::with_capacity(layer_count as usize);
    layers.resize(
        layer_count as usize,
        ffi::VkLayerProperties {
            layerName: [0; 256],
            specVersion: 0,
            implementationVersion: 0,
            description: [0; 256],
        },
    );

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

struct VulkanApp {
    window: *mut ffi::GLFWwindow,
    vk_instance: ffi::VkInstance,
    debug_messenger: ffi::VkDebugUtilsMessengerEXT,
    surface: ffi::VkSurfaceKHR,
    physical_device: ffi::VkPhysicalDevice,
    device: ffi::VkDevice,
    graphics_queue: ffi::VkQueue,
    present_queue: ffi::VkQueue,
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

        dev_create_info.enabledExtensionCount = 0;
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
        self.find_queue_families(dev).is_complete() && self.check_device_extensions_support(dev)
    }

    fn check_device_extensions_support(&self, dev: ffi::VkPhysicalDevice) -> bool {
        let req_extensions_vec: Vec<*const std::ffi::c_char> =
            vec![ffi::VK_KHR_SWAPCHAIN_EXTENSION_NAME as *const u8 as *const std::ffi::c_char];

        let mut req_extensions: HashSet<CString> = HashSet::new();
        for dev_ext in req_extensions_vec {
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
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
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
