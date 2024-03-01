mod ffi_vk {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused_imports)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/vk_bindings.rs"));

    pub fn VK_MAKE_VERSION(major: u32, minor: u32, patch: u32) -> u32 {
        (major << 22) | (minor << 12) | patch
    }
    pub fn VK_MAKE_API_VERSION(variant: u32, major: u32, minor: u32, patch: u32) -> u32 {
        (variant << 29) | (major << 22) | (minor << 12) | patch
    }
}

mod ffi_glfw {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused_imports)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/glfw_bindings.rs"));
}

use std::ffi::CString;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;

struct VulkanApp {
    window: *mut ffi_glfw::GLFWwindow,
    vk_instance: ffi_vk::VkInstance,
}

impl VulkanApp {
    fn new() -> Self {
        Self {
            window: std::ptr::null_mut(),
            vk_instance: std::ptr::null_mut(),
        }
    }

    fn init_glfw(&mut self) {
        let app_title = CString::new("Vulkan").unwrap();
        unsafe {
            ffi_glfw::glfwInit();
            ffi_glfw::glfwWindowHint(
                ffi_glfw::GLFW_CLIENT_API as i32,
                ffi_glfw::GLFW_NO_API as i32,
            );
            ffi_glfw::glfwWindowHint(ffi_glfw::GLFW_RESIZABLE as i32, ffi_glfw::GLFW_FALSE as i32);
            self.window = ffi_glfw::glfwCreateWindow(
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
        // Create instance.
        let app_name = CString::new("Vulkan Triangle").unwrap();
        let engine_name = CString::new("No Engine").unwrap();
        let app_info = ffi_vk::VkApplicationInfo {
            sType: ffi_vk::VkStructureType_VK_STRUCTURE_TYPE_APPLICATION_INFO,
            pNext: std::ptr::null_mut(),
            pApplicationName: app_name.as_ptr(),
            applicationVersion: ffi_vk::VK_MAKE_VERSION(1, 0, 0),
            pEngineName: engine_name.as_ptr(),
            engineVersion: ffi_vk::VK_MAKE_VERSION(1, 0, 0),
            apiVersion: ffi_vk::VK_MAKE_API_VERSION(0, 1, 0, 0),
        };

        // Populate VkInstanceCreateInfo.

        // First get info from glfw.
        let mut ext_count: u32 = 0;
        let exts: *mut *const std::ffi::c_char;
        unsafe {
            exts = ffi_glfw::glfwGetRequiredInstanceExtensions(std::ptr::addr_of_mut!(ext_count));
        }

        // Second populate the struct with necessary info.
        let create_info = ffi_vk::VkInstanceCreateInfo {
            sType: ffi_vk::VkStructureType_VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            pApplicationInfo: std::ptr::addr_of!(app_info),
            enabledLayerCount: 0,
            ppEnabledLayerNames: std::ptr::null(),
            enabledExtensionCount: ext_count,
            ppEnabledExtensionNames: exts,
        };

        let vk_result = unsafe {
            ffi_vk::vkCreateInstance(std::ptr::addr_of!(create_info), std::ptr::null(), std::ptr::addr_of_mut!(self.vk_instance))
        };

        if vk_result != ffi_vk::VkResult_VK_SUCCESS {
            panic!("ERROR: Failed to create vk instance!");
        }
    }

    fn main_loop(&mut self) {
        if self.window.is_null() {
            panic!("ERROR: Cannot execute main loop if window is null!");
        }

        loop {
            unsafe {
                if ffi_glfw::glfwWindowShouldClose(self.window) != 0 {
                    return;
                }
                ffi_glfw::glfwPollEvents();
            }
        }
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        if !self.vk_instance.is_null() {
            unsafe {
                ffi_vk::vkDestroyInstance(self.vk_instance, std::ptr::null());
            }
        }

        if !self.window.is_null() {
            unsafe {
                ffi_glfw::glfwDestroyWindow(self.window);
            }
        }

        unsafe {
            ffi_glfw::glfwTerminate();
        }
    }
}

fn main() {
    let mut app = VulkanApp::new();

    app.init_glfw();
    app.init_vulkan();
    app.main_loop();
}
