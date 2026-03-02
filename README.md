# kladdvara

kladdvara is my [toy](https://blog.jsbarretto.com/post/software-is-joy) virtual machine for the Little Computer 3 (LC-3) architecture.

I referenced [Justin Meiners](https://www.jmeiners.com/) and [Ryan Pendleton](https://www.ryanp.me/)'s ["Write your Own Virtual Machine"](https://www.jmeiners.com/lc3-vm/) guide to write this; many thanks to them! Try kladdvara out with their [sample programs](https://www.jmeiners.com/lc3-vm/#running-the-vm).

The machine kladdvara emulates has a couple limitations at the moment. Its only I/O is teletype, so it only supports text-based applications. Trap routines are handled directly by the emulator; they're not written to be handled by LC-3 routines. The machine also skips reserved instructions instead of panicking.

kladdvara is purely a realtime interpreter, and it doesn't validate binaries before nor during runtime. This means that invalid binaries will simply crash the program.

## installing

This project uses [Cargo](https://doc.rust-lang.org/stable/cargo/).

Run the project for dev:

```bash
cargo run
```

Or build it:

```bash
cargo build
```

## usage

kladdvara takes one argument: a path to an LC-3 binary.

Examples:

```bash
kladdvara 2048.obj
```

```bash
cargo run 2048.obj
```

## license

[GNU General Public License v3.0 or later](https://spdx.org/licenses/GPL-3.0-or-later.html). See license in [`LICENSE.md`](LICENSE.md).
