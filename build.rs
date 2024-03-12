use std::env;
use std::path::PathBuf;
use std::process::Command;

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

    let _vert_shader_out = Command::new("glslc")
        .arg("shaders/shader.vert")
        .arg("-o")
        .arg(out_path.join("vert.spv"))
        .output()
        .expect("Should be able to compile shader.vert!");

    let _frag_shader_out = Command::new("glslc")
        .arg("shaders/shader.frag")
        .arg("-o")
        .arg(out_path.join("frag.spv"))
        .output()
        .expect("Should be able to compile shader.frag!");
}
