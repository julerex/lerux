fn main() {
    let config = lerux_sddf::serial_image::client_config();
    let mut bytes = vec![0u8; std::mem::size_of_val(&config)];
    lerux_sddf::serial_image::serial_config_to_bytes(&config, &mut bytes);
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("config.bin");
    std::fs::write(&out, bytes).expect("write config.bin");
    println!("cargo:rerun-if-changed=build.rs");
}
