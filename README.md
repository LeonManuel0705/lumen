# Lumen

A hobby operating system written from scratch in Rust, built around one idea: that an OS
should feel alive. Every element on screen moves under spring physics, nothing snaps, and
the whole compositor runs on bare metal with no standard library underneath it.

Lumen boots on `x86_64`, talks directly to the hardware, and draws its own interface into
a linear framebuffer. There is no libc, no allocator from a crate registry doing the heavy
lifting, and no graphics stack. Roughly 4,200 lines of `no_std` Rust sit between the
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
* Physical frame allocator over the bootloader memory map, page tables walked and
  extended by the kernel, and a first-fit heap allocator written from scratch with
  coalescing on free

**Graphics**

* Linear framebuffer driver with a double buffered present path
* Separable box blur with a multi pass approximation of a gaussian, used for the frosted
  glass panels the interface is built from
* Alpha compositing, rounded rectangles, circles and antialiased edges written by hand
* Bitmap text rendering from a font atlas that is generated at build time by a host side
  tool and embedded into the kernel binary
* A lock screen as the boot destination: large rolling clock, the date spelled out in
  German from the CMOS date registers, and an animated unlock into the desktop
* A window compositor: windows own an RGBA surface on the heap, stack, raise on click,
  drag by the title bar, and scale in and out of existence. The glass they are made of
  reads its frosted backdrop from a blurred copy of the wallpaper rather than blurring
  the screen again every frame
* Damage tracking: a frame repaints only the regions that changed, with a clip rectangle
  carried through the drawing primitives so they narrow their loops to it instead of
  drawing pixels that will be thrown away, and a self-test that catches a region somebody
  forgot to claim

**Animation**

* Critically damped spring integrator with configurable stiffness and damping
* Tween engine with the standard easing family, so motion can be authored either way
* Scene graph that owns widget state and interpolates it every frame

**Input**

* PS/2 controller driver
* Keyboard decoding through scancode set translation
* Mouse packet decoding with button state and relative movement

**Apps**

* A small app trait: a title, an update against local pointer and key state, and a draw
  into the window's own surface
* Ball, the bouncing ball that has been the animation testbed since the beginning, with
  squash and stretch, a trail and a shadow that tightens as it falls
* Uhr, the rolling clock with seconds

## Run it

```bash
cargo run
```

That builds the kernel for `x86_64-unknown-none`, assembles a bootable BIOS disk image,
and launches QEMU at 1280x720 with the host clock passed through.

The current build boots into a lock screen. Space or a click unlocks it, and the desktop
arrives with two windows. Drag them by the title bar, click one to bring it to the front,
click the red dot to close it, and click the first two dock icons to open them again.
Inside the ball window, click the ball to throw it, space to make it jump, R to reset it.

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
| `kernel/src/mem/` | Frame allocator, page mapping, heap allocator |
| `kernel/src/wm/` | Window compositor |
| `kernel/src/apps/` | The app trait and the built-in apps |
| `runner/` | Host side tool that builds the disk image and starts QEMU |
| `tools/fontgen/` | Host side font atlas generator that emits the embedded glyph data |

## Toolchain

Rust nightly, pinned in `rust-toolchain.toml`, with `rust-src` and `llvm-tools-preview`.
The pin exists because building a kernel needs `build-std`, which is nightly only, and an
unpinned nightly breaks the build roughly once a month.

## Status

What runs today: the kernel boots, sets up its own descriptor tables and interrupts, maps
itself a 24 MiB heap, keeps the real date and time from the CMOS clock, and comes up in a
lock screen with a 96 pixel rolling clock and the German date. Space or a click plays a
0.55 second unlock into the desktop, where a compositor puts app windows on screen over a
frosted top bar and dock. Keyboard and mouse are decoded from the PS/2 controller.

Frames are driven by a 60 Hz timer, and the numbers below are frames per second alongside
the share of each frame actually spent working, at 1280x720 under emulation:

| Scene | Before damage tracking | After |
|:---|:---|:---|
| Lock screen | 60 fps, 37% busy | 60 fps, 21% busy |
| Desktop, ball bouncing in a window | 30 fps, 70% busy | 60 fps, 32% busy |
| Desktop, no windows open | full redraw every frame | 60 fps, 10% busy |

That middle row is the one that matters: 23 ms of work per frame down to 5 ms, which
is 4.4 times less, while drawing twice as many frames.

A missed damage region is invisible until somebody happens to look at the right pixel at
the right moment, so the kernel checks its own work: every two seconds it redraws the
whole scene into scratch space and compares, and reports the first disagreeing pixel over
the serial port. Set `VERIFY_EVERY` to 0 in `main.rs` to turn it off.

There is still no window resizing, no minimise, and no keyboard event queue: keys are
decoded as a handful of booleans rather than events, which is the next thing in the way.

## License

MIT
