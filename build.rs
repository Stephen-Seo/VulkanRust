use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=vulkan");
    println!("cargo:rustc-link-lib=glfw");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let glfw_vk_bindings = bindgen::Builder::default()
        .header_contents("glfw_defines", "#define GLFW_INCLUDE_VULKAN")
        .header("/usr/include/GLFW/glfw3.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate glfw bindings");

    glfw_vk_bindings
        .write_to_file(out_path.join("glfw_vk_bindings.rs"))
        .expect("Couldn't write glfw bindings!");
}
