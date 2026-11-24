fn main() {
    emit_describe();
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

fn emit_describe() {
    let manifest =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let root = manifest.join("../../..");
    println!("cargo:rerun-if-env-changed=LERUX_GIT_DESCRIBE");
    let describe = env_describe().unwrap_or_else(|| {
        git_describe(&root).unwrap_or_else(|| {
            std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string())
        })
    });
    println!("cargo:rustc-env=LERUX_GIT_DESCRIBE={describe}");
}

fn env_describe() -> Option<String> {
    accept_describe(&std::env::var("LERUX_GIT_DESCRIBE").ok()?)
}

fn git_describe(root: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["describe", "--always", "--dirty"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    accept_describe(&String::from_utf8(output.stdout).ok()?)
}

fn accept_describe(text: &str) -> Option<String> {
    let text = text.trim().strip_prefix('v').unwrap_or(text.trim());
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_graphic()) {
        return None;
    }
    Some(text.to_string())
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
        "board-pc_z97_d3h_i217",
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
