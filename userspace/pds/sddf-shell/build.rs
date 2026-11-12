fn main() {
    reject_mixed_filesystem_boards();
    reject_swift_sharing_a_build();
    reject_fetch_sharing_a_build();
    reject_web_against_filesystem();
    reject_web_against_fetch();
    reject_aarch64_web_against_other_fetch();
    reject_mixed_web_boards();
    reject_aarch64_browser_against_filesystem_and_fetch();
    reject_aarch64_browser_against_web();
    reject_riscv_browser_against_filesystem_and_fetch();
    reject_riscv_browser_against_web();
    emit_config();
}

fn reject_mixed_filesystem_boards() {
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
}

fn reject_swift_sharing_a_build() {
    #[cfg(all(
        any(feature = "board-swift_sfx14_41g", feature = "console-out"),
        any(
            feature = "board-qemu_virt_aarch64_fetch_sddf",
            feature = "board-qemu_virt_aarch64_web_sddf",
            feature = "board-qemu_virt_aarch64_browser_sddf",
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf",
            feature = "board-qemu_virt_riscv64_web_sddf",
            feature = "board-qemu_virt_riscv64_browser_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_web_sddf",
            feature = "board-x86_64_generic_fs_sddf",
        )
    ))]
    compile_error!(
        "the Swift console image does not share a build with the fetch, web, browser, or filesystem image"
    );
}

fn reject_fetch_sharing_a_build() {
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
    ))]
    compile_error!(
        "the fetch image, the web image, the browser image, and the filesystem image do not share one build"
    );
}

fn reject_web_against_filesystem() {
    #[cfg(any(
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
    ))]
    compile_error!(
        "the fetch image, the web image, the browser image, and the filesystem image do not share one build"
    );
}

fn reject_web_against_fetch() {
    #[cfg(any(
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
    ))]
    compile_error!(
        "the fetch image, the web image, the browser image, and the filesystem image do not share one build"
    );
}

fn reject_aarch64_web_against_other_fetch() {
    #[cfg(any(
        all(
            feature = "board-qemu_virt_aarch64_web_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf"
        ),
        all(
            feature = "board-qemu_virt_aarch64_web_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ),
    ))]
    compile_error!(
        "the fetch image, the web image, the browser image, and the filesystem image do not share one build"
    );
}

fn reject_mixed_web_boards() {
    #[cfg(any(
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
    ))]
    compile_error!(
        "the fetch image, the web image, the browser image, and the filesystem image do not share one build"
    );
}

fn reject_aarch64_browser_against_filesystem_and_fetch() {
    #[cfg(any(
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
    ))]
    compile_error!(
        "the fetch image, the web image, the browser image, and the filesystem image do not share one build"
    );
}

fn reject_aarch64_browser_against_web() {
    #[cfg(any(
        all(
            feature = "board-qemu_virt_riscv64_web_sddf",
            feature = "board-qemu_virt_aarch64_browser_sddf"
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
    ))]
    compile_error!(
        "the fetch image, the web image, the browser image, and the filesystem image do not share one build"
    );
}

fn reject_riscv_browser_against_filesystem_and_fetch() {
    #[cfg(any(
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
    ))]
    compile_error!(
        "the fetch image, the web image, the browser image, and the filesystem image do not share one build"
    );
}

fn reject_riscv_browser_against_web() {
    #[cfg(any(
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
}

fn emit_config() {
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
