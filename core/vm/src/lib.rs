use std::fmt::Display;
use std::io::{Error, ErrorKind, Result};

pub trait Terminal {
    fn write_character(&mut self, character: char) -> std::io::Result<()>;
    fn log(&mut self, badge: &str, message: impl Display) -> std::io::Result<()>;
    fn logln(&mut self, badge: &str, message: impl Display) -> std::io::Result<()>;
    fn poll_key(&mut self) -> Option<char>;
    fn is_interrupted(&mut self) -> bool;
}

pub struct VM<T: Terminal> {
    pub memory: Memory,
    registers: Registers,
    pub terminal: T,
    operations: [Operation<T>; 16],
    trap_routines: [TrapRoutine<T>; 6],
}

impl<T: Terminal> VM<T> {
    pub fn new(terminal: T) -> Self {
        Self {
            memory: Memory::new(),
            registers: Registers::new(),
            terminal,
            operations: [
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
            ],
            trap_routines: [
                get_character,
                output_character,
                output_string,
                prompt_input_character,
                trap_no_op,
                halt,
            ],
        }
    }

    pub fn load_image(&mut self, bytes: &[u8]) -> Result<()> {
        let mut words = bytes
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]));

        let origin = words.next().ok_or(Error::from(ErrorKind::InvalidData))? as usize;

        for (offset, word) in words.enumerate() {
            self.memory.set(origin.wrapping_add(offset), word);
        }

        Ok(())
    }

    pub fn step(&mut self) -> OperationResult {
        if !self.memory.keyboard.ready {
            if let Some(character) = self.terminal.poll_key() {
                self.memory.keyboard.push_key(character);
            }
        }

        let instruction = self
            .memory
            .get(self.registers.get(Register::ProgramCounter) as usize)?;
        let pc = self.registers.get(Register::ProgramCounter).wrapping_add(1);
        self.registers.set(Register::ProgramCounter, pc);

        let opcode = (instruction >> 12) as usize;

        self.operations[opcode](self, instruction)
    }

    // pub fn run_until_blocked(&mut self) -> OperationResult {
    //     loop {
    //         match self.step()? {
    //             Status::Continue => continue,
    //             other => return Ok(other),
    //         }
    //     }
    // }
}

const MEMORY_SIZE: usize = 0x10000;
const REGISTER_COUNT: usize = 10;

trait GetData<A, T> {
    fn get(&mut self, address: A) -> T;
}

trait SetData<A> {
    fn set(&mut self, address: A, value: u16);
}

pub struct Keyboard {
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

    pub fn push_key(&mut self, character: char) {
        self.data = character as u16;
        self.ready = true;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    fn get_status(&self) -> u16 {
        if self.is_ready() { 1 << 15 } else { 0 }
    }

    fn get_data(&mut self) -> u16 {
        self.ready = false;
        self.data
    }
}

pub struct Memory {
    data: [u16; MEMORY_SIZE],
    pub keyboard: Keyboard,
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
            Ok(self.keyboard.get_status())
        } else if address == MemoryMappedRegister::KeyboardData as usize {
            Ok(self.keyboard.get_data())
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
        let mut registers = Self {
            data: [0; REGISTER_COUNT],
        };

        registers.set(Register::Condition, Condition::Zero as u16);
        registers.set(Register::ProgramCounter, PROGRAM_COUNTER_START);

        registers
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

pub enum Status {
    Continue,
    WaitForInput,
    Halt,
}

type TrapRoutine<T> = fn(&mut VM<T>) -> OperationResult;

fn trap_no_op<T: Terminal>(_: &mut VM<T>) -> OperationResult {
    Ok(Status::Continue)
}

fn get_character<T: Terminal>(vm: &mut VM<T>) -> OperationResult {
    if vm.memory.keyboard.ready {
        let character = vm.memory.keyboard.get_data();
        vm.registers.set_with_condition(0, character);
        Ok(Status::Continue)
    } else {
        // Rewind PC so this trap instruction is retried after input
        let pc = vm.registers.get(Register::ProgramCounter).wrapping_sub(1);
        vm.registers.set(Register::ProgramCounter, pc);
        Ok(Status::WaitForInput)
    }
}

fn output_character<T: Terminal>(vm: &mut VM<T>) -> OperationResult {
    let character = vm.registers.get(0) as u8 as char;

    vm.terminal.write_character(character)?;

    Ok(Status::Continue)
}

fn output_string<T: Terminal>(vm: &mut VM<T>) -> OperationResult {
    let mut address = vm.registers.get(0);

    loop {
        let character = vm.memory.get(address as usize)? as u8 as char;

        if character == '\0' {
            break Ok(Status::Continue);
        }

        vm.terminal.write_character(character)?;

        address += 1;
    }
}

fn prompt_input_character<T: Terminal>(vm: &mut VM<T>) -> OperationResult {
    vm.terminal.log("💬", "enter a character > ")?;

    Ok(Status::Continue)
}

fn halt<T: Terminal>(vm: &mut VM<T>) -> OperationResult {
    vm.terminal.logln("✅️", "program terminated")?;

    Ok(Status::Halt)
}

const TRAP_VECTOR_OFFSET: usize = 0x20;

pub type OperationResult = Result<Status>;

type Operation<T> = fn(vm: &mut VM<T>, instruction: u16) -> OperationResult;

fn no_op<T: Terminal>(_: &mut VM<T>, _: u16) -> OperationResult {
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
fn bivariate<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> (usize, usize, u16) {
    let destination_register = word(instruction, 0b0000_111_000_0_00000) as usize;
    let source_register_1 = word(instruction, 0b0000_000_111_0_00000) as usize;
    let mode = word(instruction, 0b0000_000_000_1_00000);

    let argument = if mode == 1 {
        sign_extend(word(instruction, 0b0000_000_000_0_11111), 5)
    } else {
        let source_register_2 = word(instruction, 0b0000_000_000_0_00_111) as usize;
        vm.registers.get(source_register_2)
    };

    (destination_register, source_register_1, argument)
}

fn add<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let (dr, sr1, arg) = bivariate(vm, instruction);
    let result = vm.registers.get(sr1).wrapping_add(arg);
    vm.registers.set_with_condition(dr, result);

    Ok(Status::Continue)
}

fn and<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let (dr, sr1, arg) = bivariate(vm, instruction);
    let result = vm.registers.get(sr1) & arg;
    vm.registers.set_with_condition(dr, result);
    Ok(Status::Continue)
}

fn branch<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let test_negative = word(instruction, 0b0000_1_0_0_000000000) == 1;
    let test_zero = word(instruction, 0b0000_0_1_0_000000000) == 1;
    let test_positive = word(instruction, 0b0000_0_0_1_000000000) == 1;

    if (test_negative && vm.registers.condition_is(Condition::Negative))
        || (test_zero && vm.registers.condition_is(Condition::Zero))
        || (test_positive && vm.registers.condition_is(Condition::Positive))
    {
        let offset = sign_extend(word(instruction, 0b0000_0_0_0_111111111), 9);
        let pc = vm
            .registers
            .get(Register::ProgramCounter)
            .wrapping_add(offset);
        vm.registers.set(Register::ProgramCounter, pc);
    }

    Ok(Status::Continue)
}

fn jump<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let base = word(instruction, 0b0000_000_111_000000) as usize;
    let value = vm.registers.get(base);
    vm.registers.set(Register::ProgramCounter, value);
    Ok(Status::Continue)
}

fn jump_to_subroutine<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let pc = vm.registers.get(Register::ProgramCounter);
    vm.registers.set(7, pc);

    let mode = word(instruction, 0b0000_1_00000000000);

    if mode == 1 {
        let offset = sign_extend(word(instruction, 0b0000_0_11111111111), 11);
        vm.registers
            .set(Register::ProgramCounter, pc.wrapping_add(offset));
    } else {
        let base = word(instruction, 0b0000_0_00_111_000000) as usize;
        let base_value = vm.registers.get(base);
        vm.registers.set(Register::ProgramCounter, base_value);
    }

    Ok(Status::Continue)
}

fn load<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = vm
        .registers
        .get(Register::ProgramCounter)
        .wrapping_add(offset) as usize;
    let val = vm.memory.get(addr)?;

    vm.registers.set_with_condition(dr, val);
    Ok(Status::Continue)
}

fn load_indirect<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = vm
        .registers
        .get(Register::ProgramCounter)
        .wrapping_add(offset) as usize;
    let addr2 = vm.memory.get(addr)? as usize;
    let val = vm.memory.get(addr2)?;

    vm.registers.set_with_condition(dr, val);
    Ok(Status::Continue)
}

fn load_base_plus_offset<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000_000000) as usize;
    let base = word(instruction, 0b0000_000_111_000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_000_111111), 6);

    let addr = vm.registers.get(base).wrapping_add(offset) as usize;
    let val = vm.memory.get(addr)?;

    vm.registers.set_with_condition(dr, val);
    Ok(Status::Continue)
}

fn load_effective_address<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = vm
        .registers
        .get(Register::ProgramCounter)
        .wrapping_add(offset);
    vm.registers.set_with_condition(dr, addr);

    Ok(Status::Continue)
}

fn not<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let dr = word(instruction, 0b0000_111_000_0_00000) as usize;
    let sr = word(instruction, 0b0000_000_111_0_00000) as usize;

    let val = !vm.registers.get(sr);
    vm.registers.set_with_condition(dr, val);

    Ok(Status::Continue)
}

fn store<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let sr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = vm
        .registers
        .get(Register::ProgramCounter)
        .wrapping_add(offset) as usize;
    vm.memory.set(addr, vm.registers.get(sr));

    Ok(Status::Continue)
}

fn store_indirect<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let sr = word(instruction, 0b0000_111_000000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_111111111), 9);

    let addr = vm
        .registers
        .get(Register::ProgramCounter)
        .wrapping_add(offset) as usize;
    let addr2 = vm.memory.get(addr)? as usize;

    vm.memory.set(addr2, vm.registers.get(sr));

    Ok(Status::Continue)
}

fn store_base_plus_offset<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let sr = word(instruction, 0b0000_111_000_000000) as usize;
    let base = word(instruction, 0b0000_000_111_000000) as usize;
    let offset = sign_extend(word(instruction, 0b0000_000_000_111111), 6);

    let addr = vm.registers.get(base).wrapping_add(offset) as usize;
    vm.memory.set(addr, vm.registers.get(sr));

    Ok(Status::Continue)
}

fn trap<T: Terminal>(vm: &mut VM<T>, instruction: u16) -> OperationResult {
    let trap_vector = word(instruction, 0b0000_0000_11111111) as usize;

    let pc = vm.registers.get(Register::ProgramCounter);
    vm.registers.set(7, pc);

    vm.trap_routines[trap_vector - TRAP_VECTOR_OFFSET](vm)
}
