fn main() {
    let serial = lerux_sddf::serial_image::client_config();
    let fs = lerux_sddf::fs_image::shell_client_config();
    let serial_len = std::mem::size_of_val(&serial);
    let fs_len = std::mem::size_of_val(&fs);
    let mut bytes = vec![0u8; serial_len + fs_len];
    lerux_sddf::serial_image::serial_config_to_bytes(&serial, &mut bytes[..serial_len]);
    lerux_sddf::fs_image::fs_config_to_bytes(&fs, &mut bytes[serial_len..]);
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("config.bin");
    std::fs::write(&out, bytes).expect("write config.bin");
    println!("cargo:rerun-if-changed=build.rs");
}
