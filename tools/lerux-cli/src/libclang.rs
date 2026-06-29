use std::path::Path;

use crate::install::{install_libclang, system_libclang_present, toolchains_dir};

pub struct LibclangEnv {
    pub libclang_path: Option<String>,
    pub ld_library_path: Option<String>,
}

pub fn libclang_env(root: &Path) -> LibclangEnv {
    if !system_libclang_present() {
        let lib = toolchains_dir(root)
            .join("libclang/usr/lib/x86_64-linux-gnu/libclang-14.so.14.0.0");
        if !lib.exists() {
            let _ = install_libclang(root);
        }
    }

    let clang_root = toolchains_dir(root).join("libclang");
    let llvm_lib = clang_root.join("usr/lib/llvm-14/lib");
    if llvm_lib.is_dir() {
        let ld = clang_root.join("usr/lib/x86_64-linux-gnu");
        let mut ld_path = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        if !ld_path.is_empty() {
            ld_path = format!("{}:{}", ld.display(), ld_path);
        } else {
            ld_path = ld.display().to_string();
        }
        return LibclangEnv {
            libclang_path: Some(llvm_lib.display().to_string()),
            ld_library_path: Some(ld_path),
        };
    }

    LibclangEnv {
        libclang_path: None,
        ld_library_path: None,
    }
}

pub fn apply_libclang_env(root: &Path) {
    let env = libclang_env(root);
    // SAFETY: host build tooling mutates the current process environment only.
    unsafe {
        if let Some(path) = env.libclang_path {
            std::env::set_var("LIBCLANG_PATH", path);
        }
        if let Some(path) = env.ld_library_path {
            std::env::set_var("LD_LIBRARY_PATH", path);
        }
    }
}

pub fn print_libclang_env(root: &Path) {
    let env = libclang_env(root);
    if let Some(path) = env.libclang_path {
        println!("export LIBCLANG_PATH={path}");
    }
    if let Some(path) = env.ld_library_path {
        println!("export LD_LIBRARY_PATH={path}");
    }
}