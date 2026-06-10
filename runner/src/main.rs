use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let workspace_root = workspace_root();

    let kernel_target_dir = workspace_root.join("target").join("kernel");

    println!("\x1b[35m[lumen]\x1b[0m building kernel (release)...");
    let status = Command::new("cargo")
        .current_dir(&workspace_root)
        .env("CARGO_TARGET_DIR", &kernel_target_dir)
        .args([
            "build",
            "-p",
            "lumen-kernel",
            "--release",
            "--target",
            "x86_64-unknown-none",
            "-Z",
            "build-std=core,compiler_builtins",
            "-Z",
            "build-std-features=compiler-builtins-mem",
        ])
        .status()
        .expect("failed to run cargo build for kernel");
    assert!(status.success(), "kernel build failed");

    let kernel_elf = kernel_target_dir
        .join("x86_64-unknown-none")
        .join("release")
        .join("lumen-kernel");

    let out_dir = workspace_root.join("target").join("lumen");
    std::fs::create_dir_all(&out_dir).unwrap();
    let bios_image = out_dir.join("lumen-bios.img");

    println!("\x1b[35m[lumen]\x1b[0m assembling BIOS disk image...");
    bootloader::BiosBoot::new(&kernel_elf)
        .create_disk_image(&bios_image)
        .expect("failed to create disk image");

    println!("\x1b[35m[lumen]\x1b[0m booting in QEMU...");
    let status = Command::new("qemu-system-x86_64")
        .args([
            "-drive",
            &format!("format=raw,file={}", bios_image.display()),
            "-m",
            "256M",
            "-serial",
            "stdio",
            "-display",
            "default,show-cursor=on",
        ])
        .status()
        .expect("failed to launch qemu-system-x86_64");

    if !status.success() {
        eprintln!("qemu exited with status: {status}");
    }
}

fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().to_path_buf()
}
