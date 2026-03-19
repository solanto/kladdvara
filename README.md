# <img src="https://raw.githubusercontent.com/solanto/kladdvara/refs/heads/main/web/public/icon.svg" height="32" alt="" style="display:inline"> kladdvara

kladdvara is my [toy](https://blog.jsbarretto.com/post/software-is-joy) virtual machine for the Little Computer 3 (LC-3) architecture. Its core emulation logic is written in [Rust](https://rust-lang.org/). It's made to run in two environments: natively, atop an operating system; and in a web browser. Try it out [online](https://kladdvara.dandelion.computer)!

![A screenshot of kladdvara's pink, cake-decorated, web UI. A form allows picking or uploading a program for emulation.](https://v1.screenshot.11ty.dev/https%3A%2F%2Fkladdvara.dandelion.computer/opengraph/)

I referenced [Justin Meiners](https://www.jmeiners.com/) and [Ryan Pendleton](https://www.ryanp.me/)'s ["Write your Own Virtual Machine"](https://www.jmeiners.com/lc3-vm/) guide to write this; many thanks to them! kladdvara runs their [sample programs](https://www.jmeiners.com/lc3-vm/#running-the-vm) well.

kladdvara's virtual machine has a couple limitations at the moment. Its only I/O is teletype, so it only supports text-based applications. Trap routines are handled directly by the emulator; they're not written to be handled by an LC-3 operating system. The machine also skips reserved instructions instead of panicking.

kladdvara is purely a realtime interpreter, and it doesn't validate binaries before nor during runtime. This means that invalid binaries will simply crash the program.

## getting started

This project primarily uses [Cargo](https://doc.rust-lang.org/stable/cargo/), as well as [pnpm](https://pnpm.io/) for its web app.

### building natively

Install dependencies and build the native terminal app with Cargo, from within the [`core`](core) directory.

```bash
cd core
cargo build --bin native
```

This will place built artifacts at `core/target/debug`, including the `kladdvara` binary. Run the VM by supplying a path to a compiled LC-3 binary.

```bash
kladdvara program.obj
```

Append `--profile release` to the build command to build an optimized binary, or append—for example—`--target x86_64-unknown-linux-musl` to build a [statically-linked](https://en.wikipedia.org/wiki/Static_build) binary. Artifacts will be placed in corresponding directories within `core/target`.

Quickly build and run the project for development with a command like `cargo run program.obj`.

### building for the web

To build for the web, you'll need either to have [rustup](https://rustup.rs/) or install the Rust `wasm32-unknown-unknown` target manually. See more info [in `wasm-pack`'s docs](https://drager.github.io/wasm-pack/book/prerequisites/non-rustup-setups.html).

Install remaining dependencies and build the web terminal app with pnpm, from within the [`web`](web) directory. No need to build Rust code separately; `wasm-pack` will build `core` and `core/terminals/web` automatically as dependencies of the [Webpack](https://webpack.js.org/) project.

```bash
cd web
pnpm install
pnpm build
```

This will place built artifacts at `web/dist`. The web app runs fully in the browser; any static web server can host those files to serve an instance.

Quickly build and run the project for development with `pnpm dev`; Wepack will spin up its own dev server and hot-reload resources as frontend and core code changes.

## usage

### native

kladdvara's executable takes one argument: a path to an LC-3 binary.

```bash
kladdvara 2048.obj
```

### web

kladdvara's web app interactively lets you choose to select a sample program, provide a URL to a binary, or upload a binary.

## license

[GNU General Public License v3.0 or later](https://spdx.org/licenses/GPL-3.0-or-later.html). See license in [`LICENSE.md`](LICENSE.md).
