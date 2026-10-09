fn main() {
    let serial = lerux_sddf::serial_image::client_config();
    let net = lerux_sddf::net_image::client_config();
    let serial_len = std::mem::size_of_val(&serial);
    let net_len = std::mem::size_of_val(&net);
    let mut bytes = vec![0u8; serial_len + net_len];
    lerux_sddf::serial_image::serial_config_to_bytes(&serial, &mut bytes[..serial_len]);
    lerux_sddf::net_image::net_config_to_bytes(&net, &mut bytes[serial_len..]);
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("config.bin");
    std::fs::write(&out, bytes).expect("write config.bin");
    println!("cargo:rerun-if-changed=build.rs");
}
