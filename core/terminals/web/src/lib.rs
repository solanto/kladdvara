use core::*;

use ansi_term::Style;
use std::collections::VecDeque;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmVM {
    vm: VM<WebTerminal>,
}

type JSOperationResult = Result<&'static str, JsError>;

fn js_result_from(result: OperationResult) -> JSOperationResult {
    match result {
        Ok(status) => Ok(match status {
            Status::Continue => "continue",
            Status::WaitForInput => "wait-for-input",
            Status::Halt => "halt",
        }),
        Err(error) => Err(JsError::from(error)),
    }
}

#[wasm_bindgen]
impl WasmVM {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            vm: VM::new(WebTerminal::new()),
        }
    }

    pub fn step(&mut self) -> JSOperationResult {
        js_result_from(self.vm.step())
    }

    #[wasm_bindgen(js_name = pushKey)]
    pub fn push_key(&mut self, character: char) {
        self.vm.memory.keyboard.push_key(character);
    }

    #[wasm_bindgen(js_name = takeOutput)]
    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.vm.terminal.output_buffer)
    }

    #[wasm_bindgen(js_name = loadImage)]
    pub fn load_image(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        self.vm.load_image(bytes).map_err(JsError::from)
    }
}

pub struct WebTerminal {
    input_buffer: VecDeque<char>,
    output_buffer: String,
}

impl WebTerminal {
    fn new() -> Self {
        Self {
            input_buffer: VecDeque::new(),
            output_buffer: "".to_string(),
        }
    }
}

impl Terminal for WebTerminal {
    fn write_character(&mut self, character: char) -> std::io::Result<()> {
        if character == '\n' {
            self.output_buffer.push('\r');
            self.output_buffer.push('\n');
        } else {
            self.output_buffer.push(character);
        }

        Ok(())
    }

    fn log(&mut self, badge: &str, message: impl std::fmt::Display) -> std::io::Result<()> {
        let flag = format!("{badge} kladdvara:");
        let styled_flag = Style::new().bold().paint(flag);
        let message = format!("\r\n{styled_flag} {message}");

        self.output_buffer.push_str(&message);

        Ok(())
    }

    fn logln(&mut self, badge: &str, message: impl std::fmt::Display) -> std::io::Result<()> {
        self.log(badge, format!("{message}\r\n"))
    }

    fn poll_key(&mut self) -> Option<char> {
        self.input_buffer.pop_front()
    }

    fn is_interrupted(&mut self) -> bool {
        false
    }
}
