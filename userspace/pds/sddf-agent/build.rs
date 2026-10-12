fn main() {
    let fs = lerux_sddf::fs_image::shell_client_config();
    let net = lerux_sddf::net_image::shell_client_config();
    let fs_len = std::mem::size_of_val(&fs);
    let mut bytes = vec![0u8; fs_len + std::mem::size_of_val(&net)];
    lerux_sddf::fs_image::fs_config_to_bytes(&fs, &mut bytes[..fs_len]);
    lerux_sddf::net_image::net_config_to_bytes(&net, &mut bytes[fs_len..]);
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("config.bin");
    std::fs::write(&out, bytes).expect("write config.bin");
    println!("cargo:rerun-if-changed=build.rs");
}
