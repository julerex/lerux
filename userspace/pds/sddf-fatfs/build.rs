fn main() {
    let blk = lerux_sddf::blk_image::client_config();
    let fs = lerux_sddf::fs_image::server_config();
    let blk_len = std::mem::size_of_val(&blk);
    let fs_len = std::mem::size_of_val(&fs);
    let mut bytes = vec![0u8; blk_len + fs_len];
    lerux_sddf::blk_image::blk_config_to_bytes(&blk, &mut bytes[..blk_len]);
    lerux_sddf::fs_image::fs_config_to_bytes(&fs, &mut bytes[blk_len..]);
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("config.bin");
    std::fs::write(&out, bytes).expect("write config.bin");
    println!("cargo:rerun-if-changed=build.rs");
}
