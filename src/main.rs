use crossterm::event::{self, Event, KeyModifiers};
use crossterm::style::Stylize;
use crossterm::terminal;
use std::env;
use std::fmt::Display;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Stdout, Write, stderr, stdout};
use std::time::Duration;

fn log(out: &mut impl Write, badge: &str, message: impl Display) -> std::io::Result<()> {
    let flag = format!("{badge} kladdvara").bold();

    write!(out, "\r\n\r\n{flag}: {message}")
}

fn logln(out: &mut impl Write, badge: &str, message: impl Display) -> std::io::Result<()> {
    let message_line = format!("{message}\r\n");

    log(out, badge, message_line)
}

const MEMORY_SIZE: usize = 0x10000;
const REGISTER_COUNT: usize = 10;

trait GetData<A, T> {
    fn get(&mut self, address: A) -> T;
}

trait SetData<A> {
    fn set(&mut self, address: A, value: u16);
}

struct Keyboard {
    ready: bool,
    data: u16,
}

impl Keyboard {
    fn new() -> Self {
        Self {
            ready: false,
            data: 0,
        }
    }

    fn push_key(&mut self, c: char) {
        self.data = c as u16;
        self.ready = true;
    }

    fn read_status(&self) -> u16 {
        if self.ready { 1 << 15 } else { 0 }
    }

    fn read_data(&mut self) -> u16 {
        self.ready = false;
        self.data
    }
}

struct Memory {
    data: [u16; MEMORY_SIZE],
    keyboard: Keyboard,
}

impl Memory {
    #[inline]
    fn new() -> Self {
        Self {
            data: [0; MEMORY_SIZE],
            keyboard: Keyboard::new(),
        }
    }
}

impl GetData<usize, Result<u16>> for Memory {
    fn get(&mut self, address: usize) -> Result<u16> {
        if address == MemoryMappedRegister::KeyboardStatus as usize {
            Ok(self.keyboard.read_status())
        } else if address == MemoryMappedRegister::KeyboardData as usize {
            Ok(self.keyboard.read_data())
        } else {
            Ok(self.data[address])
        }
    }
}

impl GetData<MemoryMappedRegister, Result<u16>> for Memory {
    #[inline]
    fn get(&mut self, address: MemoryMappedRegister) -> Result<u16> {
        self.get(address as usize)
    }
}

impl SetData<usize> for Memory {
    #[inline]
    fn set(&mut self, address: usize, value: u16) {
        self.data[address] = value;
    }
}

impl SetData<MemoryMappedRegister> for Memory {
    #[inline]
    fn set(&mut self, address: MemoryMappedRegister, value: u16) {
        self.set(address as usize, value);
    }
}

const PROGRAM_COUNTER_START: u16 = 0x3000;

#[repr(usize)]
enum Register {
    ProgramCounter = 8,
    Condition = 9,
}

#[repr(usize)]
enum MemoryMappedRegister {
    KeyboardStatus = 0xFE00,
    KeyboardData = 0xFE02,
}

#[repr(u16)]
enum Condition {
    Positive = 0b001,
    Zero = 0b010,
    Negative = 0b100,
}

struct Registers {
    data: [u16; REGISTER_COUNT],
}

impl Registers {
    #[inline]
    fn new() -> Self {
        Self {
            data: [0; REGISTER_COUNT],
        }
    }

    #[inline]
    fn set_condition(&mut self, condition: Condition) {
        self.set(Register::Condition, condition as u16)
    }

    #[inline]
    fn condition_is(&self, condition: Condition) -> bool {
        self.data[Register::Condition as usize] == condition as u16
    }

    fn set_with_condition(&mut self, address: usize, value: u16) {
        self.set(address, value);

        let sign_bit = value & 0x8000;

        self.set_condition(if value == 0 {
            Condition::Zero
        } else if sign_bit == 0 {
            Condition::Positive
        } else {
            Condition::Negative
        });
    }
}

impl SetData<usize> for Registers {
    #[inline]
    fn set(&mut self, address: usize, value: u16) {
        self.data[address] = value;
    }
}

impl SetData<Register> for Registers {
    #[inline]
    fn set(&mut self, address: Register, value: u16) {
        self.set(address as usize, value);
    }
}

impl GetData<usize, u16> for Registers {
    #[inline]
    fn get(&mut self, address: usize) -> u16 {
        self.data[address]
    }
}

impl GetData<Register, u16> for Registers {
    #[inline]
    fn get(&mut self, address: Register) -> u16 {
        self.get(address as usize)
    }
}

enum Status {
    Continue,
    Halt,
}

fn handle_key_event(
    key: event::KeyEvent,
    memory: &mut Memory,
    stdout: &mut Stdout,
) -> Result<Status> {
    if let event::KeyEvent {
        modifiers: KeyModifiers::CONTROL,
        code: event::KeyCode::Char('c'),
        ..
    } = key
    {
        logln(stdout, "🛑", "program interrupted")?;
        stdout.flush()?;

        return Ok(Status::Halt);
    }

    if let event::KeyCode::Char(c) = key.code {
        memory.keyboard.push_key(c);
    }

    Ok(Status::Continue)
}

type TrapRoutine = fn(&mut Stdout, &mut Registers, &mut Memory) -> OperationResult;

fn trap_no_op(_: &mut Stdout, _: &mut Registers, _: &mut Memory) -> OperationResult {
    Ok(Status::Continue)
}

fn get_character(
    stdout: &mut Stdout,
    registers: &mut Registers,
    memory: &mut Memory,
) -> OperationResult {
    loop {
        if memory.keyboard.ready {
            let character = memory.keyboard.read_data();
            registers.set_with_condition(0, character);
            return Ok(Status::Continue);
        }

        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                return handle_key_event(key, memory, stdout);
            }
        }
    }
}

#[inline]
fn write_character_or_newline(stdout: &mut Stdout, character: char) -> std::io::Result<()> {
    if character == '\n' {
        write!(stdout, "\r\n")
    } else {
        write!(stdout, "{character}")
    }
}

fn output_character(
    stdout: &mut Stdout,
    registers: &mut Registers,
    _: &mut Memory,
) -> OperationResult {
    let character = registers.get(0) as u8 as char;

    write_character_or_newline(stdout, character)?;
    stdout.flush()?;

    Ok(Status::Continue)
}

fn output_string(
    stdout: &mut Stdout,
    registers: &mut Registers,
    memory: &mut Memory,
) -> OperationResult {
    let mut address = registers.get(0);

    loop {
        let character = memory.get(address as usize)? as u8 as char;

        if character == '\0' {
            stdout.flush()?;

            break Ok(Status::Continue);
        }

        write_character_or_newline(stdout, character)?;

        address += 1;
    }
}

fn prompt_input_character(
    stdout: &mut Stdout,
    _: &mut Registers,
    _: &mut Memory,
) -> OperationResult {
    log(stdout, "💬", "enter a character > ")?;
    stdout.flush()?;

    Ok(Status::Continue)
}

fn halt(stdout: &mut Stdout, _: &mut Registers, _: &mut Memory) -> OperationResult {
    logln(stdout, "✅️", "program terminated")?;
    stdout.flush()?;

    Ok(Status::Halt)
}

const TRAP_VECTOR_OFFSET: usize = 0x20;

const TRAP_ROUTINES: [TrapRoutine; 6] = [
    get_character,
    output_character,
    output_string,
    prompt_input_character,
    trap_no_op,
    halt,
];

type OperationResult = Result<Status>;

type Operation = fn(&mut Stdout, &mut Registers, &mut Memory, instruction: u16) -> OperationResult;

fn no_op(_: &mut Stdout, _: &mut Registers, _: &mut Memory, _: u16) -> OperationResult {
    Ok(Status::Continue)
}

fn sign_extend(value: u16, bit_count: usize) -> u16 {
    let signed = value as i16;
    let shift = 16 - bit_count;
    ((signed << shift) >> shift) as u16
}

#[inline]
fn word(value: u16, mask: u16) -> u16 {
    (value & mask) >> mask.trailing_zeros()
}

#[inline]
fn bivariate(registers: &mut Registers, instruction: u16) -> (usize, usize, u16) {
    let destination_register = word(instruction, 0b0000_111_000_0_00000) as usize;
    let source_register_1 = word(instruction, 0b0000_000_111_0_00000) as usize;
    let mode = word(instruction, 0b0000_000_000_1_00000);

    let argument = if mode == 1 {
        sign_extend(word(instruction, 0b0000_000_000_0_11111), 5)
    } else {
        let source_register_2 = word(instruction, 0b0000_000_000_0_00_111) as usize;
        registers.get(source_register_2)
    };

    (destination_register, source_register_1, argument)
}

fn add(
    _: &mut Stdout,
    registers: &mut Registers,
    _: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let (dr, sr1, arg) = bivariate(registers, instruction);
    let result = registers.get(sr1).wrapping_add(arg);
    registers.set_with_condition(dr, result);
    Ok(Status::Continue)
}

fn and(
    _: &mut Stdout,
    registers: &mut Registers,
    _: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let (dr, sr1, arg) = bivariate(registers, instruction);
    let result = registers.get(sr1) & arg;
    registers.set_with_condition(dr, result);
    Ok(Status::Continue)
}

fn branch(
    _: &mut Stdout,
    registers: &mut Registers,
    _: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let test_negative = word(instruction, 0b0000_1_0_0_000000000) == 1;
    let test_zero = word(instruction, 0b0000_0_1_0_000000000) == 1;
    let test_positive = word(instruction, 0b0000_0_0_1_000000000) == 1;

    if (test_negative && registers.condition_is(Condition::Negative))
        || (test_zero && registers.condition_is(Condition::Zero))
        || (test_positive && registers.condition_is(Condition::Positive))
    {
        let offset = sign_extend(word(instruction, 0b0000_0_0_0_111111111), 9);
        let pc = registers.get(Register::ProgramCounter).wrapping_add(offset);
        registers.set(Register::ProgramCounter, pc);
    }

    Ok(Status::Continue)
}

fn jump(
    _: &mut Stdout,
    registers: &mut Registers,
    _: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let base = word(instruction, 0b0000_000_111_000000) as usize;
    let value = registers.get(base);
    registers.set(Register::ProgramCounter, value);
    Ok(Status::Continue)
}

fn jump_to_subroutine(
    _: &mut Stdout,
    registers: &mut Registers,
    _: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let pc = registers.get(Register::ProgramCounter);
    registers.set(7, pc);

    let mode = word(instruction, 0b0000_1_00000000000);

    if mode == 1 {
        let offset = sign_extend(word(instruction, 0b0000_0_11111111111), 11);
        registers.set(Register::ProgramCounter, pc.wrapping_add(offset));
    } else {
        let base = word(instruction, 0b0000_0_00_111_000000) as usize;
        let base_value = registers.get(base);
        registers.set(Register::ProgramCounter, base_value);
    }

    Ok(Status::Continue)
}

fn load(
    _: &mut Stdout,
    registers: &mut Registers,
    memory: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = registers.get(Register::ProgramCounter).wrapping_add(offset) as usize;
    let val = memory.get(addr)?;

    registers.set_with_condition(dr, val);
    Ok(Status::Continue)
}

fn load_indirect(
    _: &mut Stdout,
    registers: &mut Registers,
    memory: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = registers.get(Register::ProgramCounter).wrapping_add(offset) as usize;
    let addr2 = memory.get(addr)? as usize;
    let val = memory.get(addr2)?;

    registers.set_with_condition(dr, val);
    Ok(Status::Continue)
}

fn load_base_plus_offset(
    _: &mut Stdout,
    registers: &mut Registers,
    memory: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000_000000) as usize;
    let base = word(instruction, 0b0000_000_111_000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_000_111111), 6);

    let addr = registers.get(base).wrapping_add(offset) as usize;
    let val = memory.get(addr)?;

    registers.set_with_condition(dr, val);
    Ok(Status::Continue)
}

fn load_effective_address(
    _: &mut Stdout,
    registers: &mut Registers,
    _: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = registers.get(Register::ProgramCounter).wrapping_add(offset);
    registers.set_with_condition(dr, addr);

    Ok(Status::Continue)
}

fn not(
    _: &mut Stdout,
    registers: &mut Registers,
    _: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000_0_00000) as usize;
    let sr = word(instruction, 0b0000_000_111_0_00000) as usize;

    let val = !registers.get(sr);
    registers.set_with_condition(dr, val);

    Ok(Status::Continue)
}

fn store(
    _: &mut Stdout,
    registers: &mut Registers,
    memory: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let sr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = registers.get(Register::ProgramCounter).wrapping_add(offset) as usize;
    memory.set(addr, registers.get(sr));

    Ok(Status::Continue)
}

fn store_indirect(
    _: &mut Stdout,
    registers: &mut Registers,
    memory: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let sr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = registers.get(Register::ProgramCounter).wrapping_add(offset) as usize;
    let addr2 = memory.get(addr)? as usize;

    memory.set(addr2, registers.get(sr));

    Ok(Status::Continue)
}

fn store_base_plus_offset(
    _: &mut Stdout,
    registers: &mut Registers,
    memory: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let sr = word(instruction, 0b0000_111_000_000000) as usize;
    let base = word(instruction, 0b0000_000_111_000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_000_111111), 6);

    let addr = registers.get(base).wrapping_add(offset) as usize;
    memory.set(addr, registers.get(sr));

    Ok(Status::Continue)
}

fn trap(
    stdout: &mut Stdout,
    registers: &mut Registers,
    memory: &mut Memory,
    instruction: u16,
) -> OperationResult {
    let trap_vector = word(instruction, 0b0000_0000_11111111) as usize;

    let pc = registers.get(Register::ProgramCounter);
    registers.set(7, pc);

    TRAP_ROUTINES[trap_vector - TRAP_VECTOR_OFFSET](stdout, registers, memory)
}

const OPERATIONS: [Operation; 16] = [
    branch,
    add,
    load,
    store,
    jump_to_subroutine,
    and,
    load_base_plus_offset,
    store_base_plus_offset,
    no_op,
    not,
    load_indirect,
    store_indirect,
    jump,
    no_op,
    load_effective_address,
    trap,
];

fn read_image_file(memory: &mut Memory, path: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let mut words = buffer
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]));

    let origin = words.next().ok_or(Error::from(ErrorKind::InvalidData))? as usize;

    for (offset, word) in words.enumerate() {
        memory.set(origin.wrapping_add(offset), word);
    }

    Ok(())
}

fn run() -> Result<()> {
    let mut memory = Memory::new();
    let mut registers = Registers::new();

    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.flush()?;

    {
        let mut args = env::args();
        args.next();

        let executable = args.next().ok_or(Error::from(ErrorKind::InvalidInput))?;
        read_image_file(&mut memory, &executable)?;
    }

    registers.set(Register::Condition, Condition::Zero as u16);
    registers.set(Register::ProgramCounter, PROGRAM_COUNTER_START);

    loop {
        if event::poll(Duration::ZERO)? {
            if let Event::Key(key) = event::read()? {
                if let Ok(Status::Halt) = handle_key_event(key, &mut memory, &mut stdout) {
                    break Ok(());
                };
            }
        }

        let instruction = memory.get(registers.get(Register::ProgramCounter) as usize)?;
        let pc = registers.get(Register::ProgramCounter).wrapping_add(1);
        registers.set(Register::ProgramCounter, pc);

        let opcode = (instruction >> 12) as usize;

        if let Status::Halt =
            OPERATIONS[opcode](&mut stdout, &mut registers, &mut memory, instruction)?
        {
            break Ok(());
        }
    }
}

fn main() {
    if let Err(error) = run() {
        let _ = logln(&mut stderr(), "⛔️", error);
        std::process::exit(1);
    }
}
