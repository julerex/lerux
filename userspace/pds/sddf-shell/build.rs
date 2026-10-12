fn main() {
    #[cfg(all(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_aarch64_fs_sddf"
    ))]
    compile_error!("the fetch image and the filesystem image both use channel 3");

    let serial = lerux_sddf::serial_image::client_config();
    let fs = lerux_sddf::fs_image::shell_client_config();
    let serial_len = std::mem::size_of_val(&serial);
    let fs_len = std::mem::size_of_val(&fs);
    #[cfg(all(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        not(feature = "board-qemu_virt_aarch64_fs_sddf")
    ))]
    let tail = {
        let net = lerux_sddf::net_image::shell_client_config();
        let mut bytes = vec![0u8; std::mem::size_of_val(&net)];
        lerux_sddf::net_image::net_config_to_bytes(&net, &mut bytes);
        bytes
    };
    #[cfg(all(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        not(feature = "board-qemu_virt_aarch64_fetch_sddf")
    ))]
    let tail = {
        let timer = lerux_sddf::timer::shell_client_config();
        let schedule = lerux_sddf::sched::fs_image_schedule();
        let timer_len = std::mem::size_of_val(&timer);
        let schedule_len = std::mem::size_of_val(&schedule);
        let mut bytes = vec![0u8; timer_len + schedule_len];
        lerux_sddf::timer::timer_config_to_bytes(&timer, &mut bytes[..timer_len]);
        lerux_sddf::sched::sched_to_bytes(&schedule, &mut bytes[timer_len..]);
        bytes
    };
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_aarch64_fs_sddf"
    )))]
    let tail = Vec::<u8>::new();
    let mut bytes = vec![0u8; serial_len + fs_len + tail.len()];
    lerux_sddf::serial_image::serial_config_to_bytes(&serial, &mut bytes[..serial_len]);
    lerux_sddf::fs_image::fs_config_to_bytes(&fs, &mut bytes[serial_len..serial_len + fs_len]);
    bytes[serial_len + fs_len..].copy_from_slice(&tail);
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("config.bin");
    std::fs::write(&out, bytes).expect("write config.bin");
    println!("cargo:rerun-if-changed=build.rs");
}
