# Lumen

A hobby operating system focused on smooth animations and motion design.
Written in Rust, runs in QEMU.

## Run

```bash
cargo run
```

That builds the kernel, assembles a BIOS disk image, and boots it in QEMU
(1280x720, real local time via `-rtc base=localtime`).

Controls: click the ball to throw it, **Space** to jump, **R** to reset.

### Environment variables

- `LUMEN_DISPLAY` — QEMU `-display` value. Default `default,show-cursor=on`;
  use `none` for headless runs.
- `LUMEN_QEMU_ARGS` — extra QEMU args appended after the defaults (later args
  win, so overrides work). Split on whitespace: arguments containing spaces
  cannot be expressed. Example:
  `LUMEN_QEMU_ARGS="-monitor unix:/tmp/lumen-mon.sock,server,nowait"`

## Layout

- `kernel/` — the Lumen kernel (no_std, target `x86_64-unknown-none`)
- `runner/` — host-side tool that builds the disk image and launches QEMU
- `tools/fontgen/` — host-side font atlas generator; regenerate the embedded
  font data (`kernel/src/gfx/font_data.rs` + `kernel/assets/*.bin`) with
  `cargo run -p fontgen` after changing fonts, sizes, or the glyph range
