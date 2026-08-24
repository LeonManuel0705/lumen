# Lumen

A hobby operating system written from scratch in Rust, built around one idea: that an OS
should feel alive. Every element on screen moves under spring physics, nothing snaps, and
the whole compositor runs on bare metal with no standard library underneath it.

Lumen boots on `x86_64`, talks directly to the hardware, and draws its own interface into
a linear framebuffer. There is no libc, no allocator from a crate registry doing the heavy
lifting, and no graphics stack. Roughly 2,800 lines of `no_std` Rust sit between the
bootloader handoff and the pixels.

## Why build this

Most hobby kernels stop at a text mode shell. The interesting problem is not printing to a
serial port, it is what happens when you want a smooth 60 frames per second interface and
the only tools you have are a raw framebuffer, a programmable interval timer, and whatever
math you are willing to write yourself. Blur, easing curves, subpixel text, and input
latency all become kernel concerns.

## What is implemented

**Boot and low level setup**

* Global descriptor table and task state segment with a dedicated double fault stack
* Interrupt descriptor table with handlers for the CPU exception range
* 8259 programmable interrupt controller remapped out of the exception vectors
* Programmable interval timer driving a monotonic tick counter for animation timing
* Real time clock read over CMOS, so the running system knows the actual date and time

**Graphics**

* Linear framebuffer driver with a double buffered present path
* Separable box blur with a multi pass approximation of a gaussian, used for the frosted
  glass panels the interface is built from
* Alpha compositing, rounded rectangles, circles and antialiased edges written by hand
* Bitmap text rendering from a font atlas that is generated at build time by a host side
  tool and embedded into the kernel binary

**Animation**

* Critically damped spring integrator with configurable stiffness and damping
* Tween engine with the standard easing family, so motion can be authored either way
* Scene graph that owns widget state and interpolates it every frame

**Input**

* PS/2 controller driver
* Keyboard decoding through scancode set translation
* Mouse packet decoding with button state and relative movement

## Run it

```bash
cargo run
```

That builds the kernel for `x86_64-unknown-none`, assembles a bootable BIOS disk image,
and launches QEMU at 1280x720 with the host clock passed through.

Controls in the current build: click the ball to throw it, space to make it jump, R to
reset the scene.

| Variable | Effect |
|:---|:---|
| `LUMEN_DISPLAY` | QEMU display backend. Defaults to `default,show-cursor=on`. Set to `none` for headless runs. |
| `LUMEN_QEMU_ARGS` | Extra QEMU arguments appended after the defaults, split on whitespace. |

## Repository layout

| Path | Contents |
|:---|:---|
| `kernel/` | The kernel itself. `no_std`, target `x86_64-unknown-none`. |
| `kernel/src/arch/` | GDT, IDT, PIC, PIT, RTC |
| `kernel/src/gfx/` | Framebuffer, blur, shapes, colour, text, font data |
| `kernel/src/anim/` | Spring integrator, tween engine, easing curves |
| `kernel/src/input/` | PS/2, keyboard, mouse |
| `runner/` | Host side tool that builds the disk image and starts QEMU |
| `tools/fontgen/` | Host side font atlas generator that emits the embedded glyph data |

## Toolchain

Rust nightly, pinned in `rust-toolchain.toml`, with `rust-src` and `llvm-tools-preview`.
The pin exists because building a kernel needs `build-std`, which is nightly only, and an
unpinned nightly breaks the build roughly once a month.

## Status

The kernel boots, keeps time, renders its interface, and handles keyboard and mouse input.
Phases zero through three of the plan are complete: bare metal boot, the graphics stack,
the animation system, and input. What comes next is memory management with a real heap,
then a window compositor, then applications running inside it.

## License

MIT
