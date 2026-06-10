# Lumen

A hobby operating system focused on smooth animations and motion design.
Written in Rust, runs in QEMU.

## Run

```bash
cargo run
```

That builds the kernel, assembles a BIOS disk image, and boots it in QEMU.

## Layout

- `kernel/` — the Lumen kernel (no_std, target `x86_64-unknown-none`)
- `runner/` — host-side tool that builds the disk image and launches QEMU
