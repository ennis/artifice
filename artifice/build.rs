extern crate gl_generator;

use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator};
use std::{env, fs::File, path::Path, path::PathBuf};

fn main() {
    let target = env::var("TARGET").unwrap();
    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());


    println!("cargo:rerun-if-changed=build.rs");

    let mut file = File::create(&dest.join("bindings.rs")).unwrap();

    Registry::new(
        Api::Gl,
        (4, 6),
        Profile::Core,
        Fallbacks::All,
        ["GL_ARB_sparse_texture"],
    )
    .write_bindings(StructGenerator, &mut file)
    .unwrap();

    if target.contains("windows") {
        let mut file =
            File::create(&dest.join("wgl_bindings.rs")).unwrap();
        Registry::new(
            Api::Wgl,
            (1, 0),
            Profile::Core,
            Fallbacks::All,
            [
                "WGL_ARB_create_context",
                "WGL_ARB_create_context_profile",
                "WGL_ARB_create_context_robustness",
                "WGL_ARB_context_flush_control",
                "WGL_ARB_extensions_string",
                "WGL_ARB_framebuffer_sRGB",
                "WGL_ARB_multisample",
                "WGL_ARB_pixel_format",
                "WGL_ARB_pixel_format_float",
                "WGL_EXT_create_context_es2_profile",
                "WGL_EXT_extensions_string",
                "WGL_EXT_framebuffer_sRGB",
                "WGL_EXT_swap_control",
                "WGL_NV_DX_interop",
                "WGL_NV_DX_interop2",
            ],
        )
        .write_bindings(gl_generator::StructGenerator, &mut file)
        .unwrap();
    }

    //embed_resource::compile("hidpi.rc");
}
