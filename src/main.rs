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

struct VulkanApp {
    window: *mut ffi::GLFWwindow,
    vk_instance: ffi::VkInstance,
    debug_messenger: ffi::VkDebugUtilsMessengerEXT,
}

impl VulkanApp {
    fn new() -> Self {
        Self {
            window: std::ptr::null_mut(),
            vk_instance: std::ptr::null_mut(),
            debug_messenger: std::ptr::null_mut(),
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

        // Create instance.
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
        let create_info = ffi::VkInstanceCreateInfo {
            sType: ffi::VkStructureType_VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            pApplicationInfo: std::ptr::addr_of!(app_info),
            enabledLayerCount: if ENABLE_VALIDATION_LAYERS {
                VALIDATION_LAYERS.len() as u32
            } else {
                0
            },
            ppEnabledLayerNames: if ENABLE_VALIDATION_LAYERS {
                VALIDATION_LAYERS.as_ptr() as *const *const i8
            } else {
                std::ptr::null()
            },
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

        self.setup_debug_messenger();
    }

    fn setup_debug_messenger(&mut self) {
        if !ENABLE_VALIDATION_LAYERS {
            return;
        }

        if self.vk_instance.is_null() {
            panic!("ERROR: Cannot set up debug messenger if vk_instance is not initialized!");
        }

        let create_info = ffi::VkDebugUtilsMessengerCreateInfoEXT {
            sType: ffi::VkStructureType_VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            pNext: std::ptr::null(),
            flags: 0,
            messageSeverity: ffi::VkDebugUtilsMessageSeverityFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_SEVERITY_VERBOSE_BIT_EXT | ffi::VkDebugUtilsMessageSeverityFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT | ffi::VkDebugUtilsMessageSeverityFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT,
            messageType: ffi::VkDebugUtilsMessageTypeFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT | ffi::VkDebugUtilsMessageTypeFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT | ffi::VkDebugUtilsMessageTypeFlagBitsEXT_VK_DEBUG_UTILS_MESSAGE_TYPE_PERFORMANCE_BIT_EXT,
            pfnUserCallback: Some(validation_debug_callback),
            pUserData: std::ptr::null_mut(),
        };

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
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
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
