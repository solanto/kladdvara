use core::*;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Stylize;
use crossterm::terminal;
use std::env;
use std::fmt::Display;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Stderr, Stdout, Write, stderr, stdout};
use std::time::Duration;

struct NativeTerminal {
    stdout: Stdout,
    stderr: Stderr,
    interrupted: bool,
}

impl NativeTerminal {
    fn new() -> Self {
        Self {
            stdout: stdout(),
            stderr: stderr(),
            interrupted: false,
        }
    }
}

#[inline]
fn is_interrupt(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
    )
}

impl Terminal for NativeTerminal {
    fn write_character(&mut self, character: char) -> std::io::Result<()> {
        if character == '\n' {
            write!(self.stdout, "\r\n")
        } else {
            write!(self.stdout, "{character}")
        }
    }

    fn log(&mut self, badge: &str, message: impl Display) -> std::io::Result<()> {
        let flag = format!("{badge} kladdvara").bold();

        write!(self.stderr, "\r\n{flag}: {message}")
    }

    fn logln(&mut self, badge: &str, message: impl Display) -> std::io::Result<()> {
        self.log(badge, format!("{message}\r\n"))
    }

    fn poll_key(&mut self) -> Option<char> {
        if event::poll(Duration::ZERO).ok()? {
            if let Event::Key(key) = event::read().ok()? {
                if is_interrupt(key) {
                    self.interrupted = true;

                    return None;
                } else if let KeyCode::Char(character) = key.code {
                    return Some(character);
                }
            }
        }

        None
    }

    fn is_interrupted(&mut self) -> bool {
        self.interrupted
    }
}

fn interrupt(vm: &mut VM<NativeTerminal>) -> Result<()> {
    vm.terminal.logln("🛑", "program interrupted")?;

    Ok(())
}

fn run(vm: &mut VM<NativeTerminal>) -> Result<()> {
    terminal::enable_raw_mode()?;
    vm.terminal.stdout.flush()?;
    vm.terminal.stderr.flush()?;

    {
        let mut args = env::args();
        args.next();

        let path = args.next().ok_or(Error::from(ErrorKind::InvalidInput))?;
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;

        vm.load_image(&buffer)?
    }

    loop {
        match vm.step()? {
            Status::Halt => break Ok(()),
            Status::WaitForInput => loop {
                if let Event::Key(key) = event::read()? {
                    if is_interrupt(key) {
                        return interrupt(vm);
                    } else if let KeyCode::Char(character) = key.code {
                        vm.memory.keyboard.push_key(character);

                        break;
                    }
                }
            },
            Status::Continue => {}
        }

        if vm.terminal.is_interrupted() {
            break interrupt(vm);
        }

        vm.terminal.stdout.flush()?;
        vm.terminal.stderr.flush()?;
    }
}

fn main() {
    let mut vm = VM::new(NativeTerminal::new());

    if let Err(error) = run(&mut vm) {
        let _ = vm.terminal.logln("⛔️", error);
        std::process::exit(1);
    }
}
