fn main() {
    write_config();
    write_modules();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/smoke.wat");
    println!("cargo:rerun-if-changed=../../../support/keys/smoke.ed25519");
}

fn write_config() {
    let serial = lerux_sddf::serial_image::client_config();
    let fs = lerux_sddf::fs_image::shell_client_config();
    let net = lerux_sddf::net_image::shell_client_config();
    let serial_len = std::mem::size_of_val(&serial);
    let fs_len = std::mem::size_of_val(&fs);
    let net_len = std::mem::size_of_val(&net);
    let mut bytes = vec![0u8; serial_len + fs_len + net_len];
    lerux_sddf::serial_image::serial_config_to_bytes(&serial, &mut bytes[..serial_len]);
    lerux_sddf::fs_image::fs_config_to_bytes(&fs, &mut bytes[serial_len..serial_len + fs_len]);
    lerux_sddf::net_image::net_config_to_bytes(&net, &mut bytes[serial_len + fs_len..]);
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("config.bin");
    std::fs::write(&out, bytes).expect("write config.bin");
}

fn write_modules() {
    use ed25519_dalek::Signer;

    let wat = std::fs::read_to_string("src/smoke.wat").expect("read smoke.wat");
    let wasm = wat::parse_str(&wat).expect("parse smoke.wat");
    let key_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../support/keys/smoke.ed25519");
    let secret = std::fs::read(&key_path).expect("read smoke key");
    let secret: [u8; 32] = secret.as_slice().try_into().expect("smoke key is 32 bytes");
    let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
    let mut message = vec![0u8; lerux_wasm::HEADER_LEN + wasm.len()];
    let message_len =
        lerux_wasm::signing_message(&wasm, &mut message).expect("wasm fits in the envelope");
    let signature = signing.sign(&message[..message_len]);
    let mut sig = [0u8; lerux_wasm::SIG_LEN];
    sig.copy_from_slice(&signature.to_bytes());
    let mut blob = vec![0u8; message_len + lerux_wasm::SIG_LEN];
    let total = lerux_wasm::encode(&wasm, &sig, &mut blob).expect("encode blob");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(out_dir.join("smoke.lrw"), &blob[..total]).expect("write smoke.lrw");
    let mut bad = blob[..total].to_vec();
    let last = bad.len() - 1;
    bad[last] ^= 0x01;
    std::fs::write(out_dir.join("smoke-bad.lrw"), bad).expect("write smoke-bad.lrw");
}
