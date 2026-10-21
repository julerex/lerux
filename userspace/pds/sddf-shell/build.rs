fn main() {
    #[cfg(any(
        all(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            any(
                feature = "board-qemu_virt_riscv64_fs_sddf",
                feature = "board-x86_64_generic_fs_sddf"
            )
        ),
        all(
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    ))]
    compile_error!("filesystem board features are mutually exclusive");

    #[cfg(any(
        all(
            feature = "board-qemu_virt_aarch64_fetch_sddf",
            any(
                feature = "board-qemu_virt_aarch64_fs_sddf",
                feature = "board-qemu_virt_riscv64_fs_sddf",
                feature = "board-x86_64_generic_fs_sddf",
                feature = "board-qemu_virt_riscv64_fetch_sddf",
                feature = "board-x86_64_generic_fetch_sddf"
            )
        ),
        all(
            feature = "board-qemu_virt_riscv64_fetch_sddf",
            any(
                feature = "board-qemu_virt_aarch64_fs_sddf",
                feature = "board-qemu_virt_riscv64_fs_sddf",
                feature = "board-x86_64_generic_fs_sddf",
                feature = "board-qemu_virt_aarch64_fetch_sddf",
                feature = "board-x86_64_generic_fetch_sddf"
            )
        ),
        all(
            feature = "board-x86_64_generic_fetch_sddf",
            any(
                feature = "board-qemu_virt_aarch64_fs_sddf",
                feature = "board-qemu_virt_riscv64_fs_sddf",
                feature = "board-x86_64_generic_fs_sddf",
                feature = "board-qemu_virt_aarch64_fetch_sddf",
                feature = "board-qemu_virt_riscv64_fetch_sddf",
                feature = "board-x86_64_generic_web_sddf"
            )
        ),
        all(
            feature = "board-qemu_virt_aarch64_web_sddf",
            any(
                feature = "board-qemu_virt_aarch64_fs_sddf",
                feature = "board-qemu_virt_riscv64_fs_sddf",
                feature = "board-x86_64_generic_fs_sddf"
            )
        ),
        all(
            feature = "board-qemu_virt_riscv64_web_sddf",
            any(
                feature = "board-qemu_virt_aarch64_fs_sddf",
                feature = "board-qemu_virt_riscv64_fs_sddf",
                feature = "board-x86_64_generic_fs_sddf"
            )
        ),
        all(
            feature = "board-x86_64_generic_web_sddf",
            any(
                feature = "board-qemu_virt_aarch64_fs_sddf",
                feature = "board-qemu_virt_riscv64_fs_sddf",
                feature = "board-x86_64_generic_fs_sddf"
            )
        ),
        all(
            feature = "board-qemu_virt_aarch64_web_sddf",
            feature = "board-qemu_virt_aarch64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_web_sddf",
            feature = "board-qemu_virt_aarch64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_web_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_web_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ),
        all(
            feature = "board-x86_64_generic_web_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ),
        all(
            feature = "board-x86_64_generic_web_sddf",
            feature = "board-qemu_virt_aarch64_fetch_sddf"
        ),
        all(
            feature = "board-x86_64_generic_web_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_web_sddf",
            feature = "board-x86_64_generic_web_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_web_sddf",
            feature = "board-x86_64_generic_web_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_web_sddf",
            feature = "board-qemu_virt_riscv64_web_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_browser_sddf",
            any(
                feature = "board-qemu_virt_aarch64_fs_sddf",
                feature = "board-qemu_virt_riscv64_fs_sddf",
                feature = "board-x86_64_generic_fs_sddf"
            )
        ),
        all(
            feature = "board-qemu_virt_aarch64_browser_sddf",
            feature = "board-qemu_virt_aarch64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_browser_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_web_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_web_sddf",
            feature = "board-qemu_virt_aarch64_browser_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_web_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_browser_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_browser_sddf",
            feature = "board-x86_64_generic_web_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_browser_sddf",
            feature = "board-qemu_virt_aarch64_web_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_browser_sddf",
            feature = "board-qemu_virt_riscv64_web_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_browser_sddf",
            feature = "board-qemu_virt_riscv64_browser_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_browser_sddf",
            any(
                feature = "board-qemu_virt_aarch64_fs_sddf",
                feature = "board-qemu_virt_riscv64_fs_sddf",
                feature = "board-x86_64_generic_fs_sddf"
            )
        ),
        all(
            feature = "board-qemu_virt_riscv64_browser_sddf",
            feature = "board-qemu_virt_aarch64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_browser_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_web_sddf",
            feature = "board-qemu_virt_riscv64_browser_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_browser_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_browser_sddf",
            feature = "board-x86_64_generic_web_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_browser_sddf",
            feature = "board-qemu_virt_aarch64_web_sddf"
        ),
        all(
            feature = "board-qemu_virt_riscv64_browser_sddf",
            feature = "board-qemu_virt_riscv64_web_sddf"
        ),
    ))]
    compile_error!(
        "the fetch image, the web image, the browser image, and the filesystem image do not share one build"
    );

    let mut features = Vec::new();
    for feature in [
        "board-qemu_virt_aarch64_fs_sddf",
        "board-qemu_virt_riscv64_fs_sddf",
        "board-x86_64_generic_fs_sddf",
        "board-qemu_virt_aarch64_fetch_sddf",
        "board-qemu_virt_riscv64_fetch_sddf",
        "board-x86_64_generic_fetch_sddf",
        "board-x86_64_generic_web_sddf",
        "board-qemu_virt_aarch64_web_sddf",
        "board-qemu_virt_riscv64_web_sddf",
        "board-qemu_virt_aarch64_browser_sddf",
        "board-qemu_virt_riscv64_browser_sddf",
        "net-client",
    ] {
        let key = format!(
            "CARGO_FEATURE_{}",
            feature.to_ascii_uppercase().replace('-', "_")
        );
        if std::env::var_os(key).is_some() {
            features.push(feature);
        }
    }
    let out =
        std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("config.bin");
    lerux_image::emit_pd_binding("sddf-shell", &features, &out).expect("config.bin");
    println!("cargo:rerun-if-changed=build.rs");
}
