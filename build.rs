use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=vulkan");
    println!("cargo:rustc-link-lib=glfw");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let vk_bindings = bindgen::Builder::default()
        .header("/usr/include/vulkan/vulkan.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate vulkan bindings");

    vk_bindings
        .write_to_file(out_path.join("vk_bindings.rs"))
        .expect("Couldn't write vk bindings!");

    let glfw_bindings = bindgen::Builder::default()
        .header_contents("glfw_defines", "#define GLFW_INCLUDE_VULKAN")
        .header("/usr/include/GLFW/glfw3.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate glfw bindings");

    glfw_bindings
        .write_to_file(out_path.join("glfw_bindings.rs"))
        .expect("Couldn't write glfw bindings!");
}
