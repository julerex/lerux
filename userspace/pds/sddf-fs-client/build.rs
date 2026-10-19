fn main() {
    let out =
        std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("config.bin");
    lerux_image::emit_pd_binding("sddf-fs-client", &[], &out).expect("config.bin");
    println!("cargo:rerun-if-changed=build.rs");
}
