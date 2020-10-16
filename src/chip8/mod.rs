use std::num::Wrapping;
use device_query::Keycode;
use rand::Rng;

pub(crate) struct Chip8 {
    memory: [u8; 4096],
    // V
    cpu_registers: [Wrapping<u8>; 16],
    // I
    index_register: Wrapping<u16>,
    // Increment by 2 as each instruction is 2 bytes long
    // True if we do not call subroutine or jump to a certain address in memory
    // Will increment by four if next opcode should be skipped
    program_counter: u16,
    gfx: [u8; 64 * 32],
    delay_timer: u8,
    sound_timer: u8,
    stack: [u16; 16],
    stack_pointer: u16,
    keys: [u8; 16],
    draw_flag: bool,
}

const CHIP8_FONTSET: [u8; 80] = [0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

impl Chip8 {
    pub fn new() -> Self {
        // Initialize registers and memory once
        let mut new_chip8 = Chip8 {
            memory: [0; 4096],
            cpu_registers: [Wrapping(0); 16],
            index_register: Wrapping(0),
            program_counter: 0x200,
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            stack_pointer: 0,
            keys: [0; 16],
            draw_flag: false,
            gfx: [0; 64 * 32],
        };

        // Load fontset
        for i in 0..CHIP8_FONTSET.len() {
            new_chip8.memory[i] = CHIP8_FONTSET[i];
        }

        new_chip8
    }

    pub fn emulate_cycle(&mut self) {
        // Fetch Opcode
        let opcode: u16 = (self.memory[self.program_counter as usize] as u16) << 8
            | (self.memory[self.program_counter as usize + 1] as u16);

        let command_bit: u8 = ((opcode & 0xF000) >> 12) as u8;

        let v_x: usize = ((opcode & 0x0F00) >> 8) as usize;
        let v_y: usize = ((opcode & 0x00F0) >> 4) as usize;
        let nn = (opcode & 0x00FF) as u8;
        let nnn = opcode & 0x0FFF;

        // Decode and Execute Opcode
        // Note: "NNN" denotes last three "nibbles" of two-byte opcode
        // "NN" denotes last two "nibbles" of two-byte opcode
        match command_bit {
            // Calls machine code at address NNN
            0x0 => {
                match opcode {
                    0x00E0 => self.clear_screen(),
                    0x00EE => self.return_from_subroutine(),
                    _ => panic!("Unknown opcode: {:#X}", opcode),
                }
            }
            0x1 => self.process_1_command(nnn),
            0x2 => self.process_2_command(nnn),
            0x3 => self.process_3_command(v_x, nn),
            0x4 => self.process_4_command(v_x, nn),
            0x5 => {
                match opcode & 0x000F {
                    0x0000 => self.process_5_command(v_x, v_y),
                    _ => panic!("Unknown opcode: {:#X}", opcode),
                }
            },
            0x6 => self.process_6_command(v_x, nn),
            0x7 => self.process_7_command(v_x, nn),
            0x8 => self.process_8_command(opcode & 0x000F, v_x, v_y),
            0x9 => {
                match opcode & 0x000F {
                    0x0000 => self.process_9_command(v_x, v_y),
                    _ => panic!("Unknown opcode: {:#X}", opcode),
                }
            },
            0xA => self.process_a_command(nnn),
            0xB => self.process_b_command(nnn),
            0xC => self.process_c_command(v_x, nn),
            // Draw sprite at coordinate (VX, VY) 8 pixels wide and N pixels high where N is last nibble
            0xD => {
                // Fetch position and height of sprite
                let x = self.cpu_registers[v_x].0 as u16;
                let y = self.cpu_registers[v_y].0 as u16;
                // Pixel value
                let height: u16 = opcode & 0x000F;

                // Reset register VF
                self.cpu_registers[0x0F] = Wrapping(0);
                for y_line in 0..height {
                    // fetch pixel value from memory starting at location I
                    let pixel = self.memory[(self.index_register.0 + y_line) as usize];
                    // Sprite is always 8 wide, loop over 8 bits to draw one row
                    for x_line in 0..8 {
                        // Check if current pixel is set to 1 (using >> x_line to scan through byte)
                        if (pixel & (0x80 >> x_line)) != 0 {
                            let gfx_idx: usize = ((x + x_line + ((y + y_line) * 64)) as usize) % self.gfx.len();

                            // If current pixel is 1 we need to set the VF register
                            if self.gfx[gfx_idx] == 1 {
                                self.cpu_registers[0x0F] = Wrapping(1);
                            }
                            // Set pixel value using XOR
                            self.gfx[gfx_idx] ^= 1;
                        }
                    }
                }

                // gfx array updated, need to draw screen
                self.draw_flag = true;
                // Move to next opcode
                self.program_counter += 2;
            },
            0xE => {
                match opcode & 0x00FF {
                    0x009E => self.process_ex9e_command(v_x),
                    0x00A1 => self.process_exa1_command(v_x),
                    _ => panic!("Unknown opcode: {:#X}", opcode),
                }
            },
            0xF => {
                match opcode & 0xF0FF {
                    // Store current value of delay timer in register VX
                    0xF007 => {
                        self.cpu_registers[v_x] = Wrapping(self.delay_timer);
                        self.program_counter += 2;
                    }
                    // Set delay timer to value of register VX
                    0xF015 => {
                        self.delay_timer = self.cpu_registers[v_x].0;
                        self.program_counter += 2;
                    }
                    // Set sound timer to VX
                    0xF018 => {
                        self.sound_timer = self.cpu_registers[v_x].0;
                        self.program_counter += 2;
                    }
                    // 0xFX1E - Adds VX to I. VF not affected
                    0xF01E => {
                        self.index_register += Wrapping(self.cpu_registers[v_x].0 as u16);
                        self.program_counter += 2;
                    }
                    // Sets I to location of the sprite for character in VX
                    0xF029 => {
                        self.index_register = Wrapping((self.cpu_registers[v_x].0 as u16) * 5);
                        self.program_counter += 2;
                    }
                    // Store binary-coded decimal representation of VX at addresses I, I+1, and I+2
                    0xF033 => { // opcode 0xFX33
                        self.memory[self.index_register.0 as usize] = self.cpu_registers[v_x].0 / 100;
                        self.memory[self.index_register.0 as usize + 1] = (self.cpu_registers[v_x].0 / 10) % 10;
                        self.memory[self.index_register.0 as usize + 2] = (self.cpu_registers[v_x].0 % 100) % 10;
                        self.program_counter += 2;
                    }
                    // Stores V0 to VX in memory starting at address I
                    0xF055 => {
                        for i in 0..v_x + 1 {
                            self.memory[self.index_register.0 as usize + i] = self.cpu_registers[i].0;
                        }
                        self.program_counter += 2;
                    }
                    // Fills V0 to VX (including VX) with values from memory starting at address I
                    0xF065 => {
                        for i in 0..v_x + 1 {
                            self.cpu_registers[i] = Wrapping(self.memory[self.index_register.0 as usize + i]);
                        }
                        self.program_counter += 2;
                    }
                    _ => panic!("Unknown opcode: {:#X}", opcode),
                }
            }
            _ => panic!("Unknown opcode: {:#X}", opcode),
        }

        // Update timers
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                println!("BEEP");
            }
            self.sound_timer -= 1;
        }
    }

    /// 0x00E0
    /// Clear the screen of all sprite data
    fn clear_screen(&mut self) {
        self.gfx = [0; 64 * 32];
        self.draw_flag = true;
        self.program_counter += 2;
    }

    /// 0x00EE
    /// Return from subroutine
    /// Stack pointer is decremented and program counter is set back to value retrieved from stack
    fn return_from_subroutine(&mut self) {
        self.program_counter = self.stack[self.stack_pointer as usize] + 2;
        self.stack_pointer -= 1;
    }

    /// 0x1NNN
    /// Program counter jumps to address NNN
    fn process_1_command(&mut self, nnn: u16) {
        self.program_counter = nnn;
    }

    /// 0x2nnn
    /// Calls subroutine at NNN
    fn process_2_command(&mut self, nnn: u16) {
        // Store current position of program counter on the stack
        self.stack_pointer += 1;
        self.stack[self.stack_pointer as usize] = self.program_counter;
        // Set program counter to nnn to start subroutine
        self.program_counter = nnn;
    }

    /// 0x3XNN
    /// Skip next instruction if VX equals NN
    fn process_3_command(&mut self, v_x: usize, nn: u8) {
        self.program_counter += if self.cpu_registers[v_x].0 == nn { 4 } else { 2 };
    }

    /// 0x4XNN
    /// Skip next instruction if VX does NOT equals NN
    fn process_4_command(&mut self, v_x: usize, nn: u8) {
        self.program_counter += if self.cpu_registers[v_x].0 != nn { 4 } else { 2 };
    }

    /// 0x5NNN
    /// Determine if opcode is 0x5XY0
    /// If so, skip next instruction if VX = VY
    fn process_5_command(&mut self, v_x: usize, v_y: usize) {
        self.program_counter += if self.cpu_registers[v_x] == self.cpu_registers[v_y] { 4 } else { 2 };
    }

    /// 0x6XNN
    /// Sets VX to NN
    fn process_6_command(&mut self, v_x: usize, nn: u8) {
        self.cpu_registers[v_x] = Wrapping(nn);
        self.program_counter += 2;
    }

    /// 0x7XNN
    /// Adds NN to VX
    fn process_7_command(&mut self, v_x: usize, nn: u8) {
        self.cpu_registers[v_x] += Wrapping(nn);
        self.program_counter += 2;
    }

    /// 0x8XYN
    /// Various arithmetic instructions
    fn process_8_command(&mut self, operator: u16, v_x: usize, v_y: usize) {
        match operator {
            // 0x8XY0 - Sets VX to the value of VY
            0x0000 => {
                self.cpu_registers[v_x] = self.cpu_registers[v_y];
                self.program_counter += 2;
            }
            // 0x8XY1 - Sets VX to bitwise OR operation of VX and VY
            0x0001 => {
                self.cpu_registers[v_x] |= self.cpu_registers[v_y];
                self.program_counter += 2;
            }
            // 0x8XY2 - Sets VX to bitwise AND operation of VX and VY
            0x0002 => {
                self.cpu_registers[v_x] &= self.cpu_registers[v_y];
                self.program_counter += 2;
            }
            // 0x8XY3 - Sets VX to bitwise XOR operation of VX and VY
            0x0003 => {
                self.cpu_registers[v_x] ^= self.cpu_registers[v_y];
                self.program_counter += 2;
            }
            // 0x8XY4 - Adds value of VY to VX
            0x0004 => {
                self.cpu_registers[0xF] = Wrapping(match self.cpu_registers[v_x].0 > (0xFF - self.cpu_registers[v_y].0) {
                    true => 1, // carry
                    false => 0
                });

                self.cpu_registers[v_x] += self.cpu_registers[v_y];
                self.program_counter += 2;
            }
            // 0x8XY5 - Sets VX to VX - VY. VF set to 0 when there's borrow, 1 when there isn't
            0x0005 => {
                self.cpu_registers[0xF] = Wrapping(if self.cpu_registers[v_y] > self.cpu_registers[v_x] {
                    0x00 // Borrow occurred
                } else {
                    0x01
                });
                self.cpu_registers[v_x] -= self.cpu_registers[v_y];
                self.program_counter += 2;
            }
            // 0x8XY6 - Store least significant bit of VS in VF and then shifts VX to the right by 1
            0x0006 => {
                self.cpu_registers[0x0F] = Wrapping(self.cpu_registers[v_x].0 & 1);
                self.cpu_registers[v_x] >>= 1;
                self.program_counter += 2;
            }
            // 0x08XY7 - Sets VX to VY - VX. VF set to 0 when there's a borrow and 1 when there isn't
            0x0007 => {
                self.cpu_registers[0xF] = Wrapping(if self.cpu_registers[v_x] > self.cpu_registers[v_y] {
                    0x00 // Borrow occurred
                } else {
                    0x01
                });
                self.cpu_registers[v_x] = self.cpu_registers[v_y] - self.cpu_registers[v_x];
                self.program_counter += 2;
            }
            // 0x8XYE - Store most significant bit of VX in VF and then shifts VX to the left by 1
            0x000E => {
                self.cpu_registers[0x0F] = Wrapping((self.cpu_registers[v_x].0 & 0b10000000) >> 7);
                self.cpu_registers[v_x] <<= 1;
                self.program_counter += 2;
            }
            _ => panic!("Unknown opcode: {:#X}", operator),
        }
    }

    /// 0x9XY0
    /// Skips next instruction if VX doesn't equal VY (program counter increments by 4 instead of 2)
    fn process_9_command(&mut self, v_x: usize, v_y: usize) {
        self.program_counter += if self.cpu_registers[v_x] != self.cpu_registers[v_y] { 4 } else { 2 };
    }

    /// 0xANNN
    /// Sets index register (I) to address NNN
    fn process_a_command(&mut self, nnn: u16) {
        self.index_register = Wrapping(nnn);
        self.program_counter += 2;
    }

    /// 0xBNNN
    /// Sets program counter to address NNN plus value of V0
    fn process_b_command(&mut self, nnn: u16) {
        self.program_counter = nnn + self.cpu_registers[0x0].0 as u16;
    }

    /// 0xCNNN
    /// Sets VX to the result of bitwise AND on random number (0 to 255) and NN
    fn process_c_command(&mut self, v_x: usize, nn: u8) {
        self.cpu_registers[v_x] = Wrapping(rand::thread_rng().gen::<u8>() & nn);
        self.program_counter += 2;
    }

    /// 0xEX9E
    /// Skips next instruction if key stored in VX is pressed
    fn process_ex9e_command(&mut self, v_x: usize) {
        let key_idx = self.cpu_registers[v_x].0 as usize;
        self.program_counter += if self.keys[key_idx] == 1 { 4 } else { 2 };
    }

    /// 0xEXA1
    /// Skips next instruction if key stored in VX is NOT pressed
    fn process_exa1_command(&mut self, v_x: usize) {
        let key_idx = self.cpu_registers[v_x].0 as usize;
        self.program_counter += if self.keys[key_idx] != 1 { 4 } else { 2 };
    }

    pub fn draw_to_buffer(&mut self, buffer: &mut Vec<u32>) -> bool {
        let mut should_draw = false;
        if self.draw_flag {
            for pixel_idx in 0..buffer.len() {
                buffer[pixel_idx] = if self.gfx[pixel_idx] == 0 { 0x0000 } else { 0x0FFF };
            }
            should_draw = true;
        }
        self.draw_flag = false;
        should_draw
    }

    pub fn set_keys(&mut self, keys: Vec<Keycode>) {
        for key in self.keys.iter_mut() {
            *key = 0;
        }

        for key in keys {
            match key {
                Keycode::Key1 => self.keys[0] = 1,
                Keycode::Key2 => self.keys[1] = 1,
                Keycode::Key3 => self.keys[2] = 1,
                Keycode::Key4 => self.keys[3] = 1,
                Keycode::Q => self.keys[4] = 1,
                Keycode::W => self.keys[5] = 1,
                Keycode::E => self.keys[6] = 1,
                Keycode::R => self.keys[7] = 1,
                Keycode::A => self.keys[8] = 1,
                Keycode::S => self.keys[9] = 1,
                Keycode::D => self.keys[10] = 1,
                Keycode::F => self.keys[11] = 1,
                Keycode::Z => self.keys[12] = 1,
                Keycode::X => self.keys[13] = 1,
                Keycode::C => self.keys[14] = 1,
                Keycode::V => self.keys[15] = 1,
                _ => {}
            }
        }
    }

    pub fn load_program(&mut self, program_buffer: &Vec<u8>) {
        for i in 0..program_buffer.len() {
            self.memory[i + 512] = program_buffer[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::chip8::Chip8;
    use std::num::Wrapping;
    use device_query::Keycode;

    fn get_chip_8(command_to_test: Option<u16>) -> Chip8 {
        let mut mock_chip = Chip8::new();
        if let Some(command_to_test) = command_to_test {
            let upper_bits = ((command_to_test & 0xFF00) >> 8) as u8;
            let lower_bits = (command_to_test & 0x00FF) as u8;
            let program_buffer: Vec<u8> = vec![upper_bits, lower_bits];
            mock_chip.load_program(&program_buffer);
        }
        mock_chip
    }

    /// Overall test of generic functionality
    /// Base program with simple jump command should load, emulate once, and program counter
    /// will have updated
    #[test]
    fn test_general_load_and_emulate_one_cycle() {
        let mut mock_chip8 = get_chip_8(Some(0x124E));
        assert_eq!(mock_chip8.program_counter, 0x0200);
        mock_chip8.emulate_cycle();
        assert_eq!(mock_chip8.program_counter, 0x024E);
    }

    /// Test goto address
    #[test]
    fn test_1nnn() {
        let mut mock_chip8 = get_chip_8(None);
        assert_eq!(mock_chip8.program_counter, 0x0200);
        mock_chip8.process_1_command(0x011E);
        assert_eq!(mock_chip8.program_counter, 0x11E);
    }

    /// Test goto for subroutine
    /// Same as #test_1nnn but stack_pointer and stack will also update
    #[test]
    fn test_2nnn() {
        let mut mock_chip8 = get_chip_8(None);
        assert_eq!(mock_chip8.program_counter, 0x0200);
        assert_eq!(mock_chip8.stack_pointer, 0);
        assert_eq!(mock_chip8.stack[1], 0);
        mock_chip8.process_2_command(0x0EEE);
        assert_eq!(mock_chip8.program_counter, 0xEEE);
        assert_eq!(mock_chip8.stack_pointer, 1);
        assert_eq!(mock_chip8.stack[1], 0x0200);
    }

    /// 0x3XNN - Test skipping next instruction
    /// Register set to be equal to register VX, program counter will increment by 4
    #[test]
    fn test_3nnn_skip() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0x14);
        assert_eq!(mock_chip8.program_counter, 0x0200);
        mock_chip8.process_3_command(0, 0x14);
        assert_eq!(mock_chip8.program_counter, 0x0200 + 4);
    }

    /// 0x3XNN - Test not skipping instruction
    /// Register set to not be equal to register VX, program counter will increment by 2
    #[test]
    fn test_3nnn_dont_skip() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0x13);
        assert_eq!(mock_chip8.program_counter, 0x0200);
        mock_chip8.process_3_command(0, 0x14);
        assert_eq!(mock_chip8.program_counter, 0x0200 + 2);
    }

    /// 0x4XNN - Test skipping next instruction
    /// Register set to not be equal to register VX, program counter will increment by 4
    #[test]
    fn test_4nnn_skip() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0xFF);
        assert_eq!(mock_chip8.program_counter, 0x0200);
        mock_chip8.process_4_command(0, 0x14);
        assert_eq!(mock_chip8.program_counter, 0x0200 + 4);
    }

    /// 0x4XNN - Test not skipping instruction
    /// Register set to be equal to register VX, program counter will increment by 2
    #[test]
    fn test_4nnn_dont_skip() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0xFF);
        assert_eq!(mock_chip8.program_counter, 0x0200);
        mock_chip8.process_4_command(0, 0xFF);
        assert_eq!(mock_chip8.program_counter, 0x0200 + 2);
    }

    /// 0x5XY0 - Test skipping instruction if V_X = V_Y
    /// Registers 0 and 1 set equal to each other, program counter will increment by 4
    #[test]
    fn test_5xy0_skip() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0xFF);
        mock_chip8.cpu_registers[1] = Wrapping(0xFF);
        assert_eq!(mock_chip8.program_counter, 0x0200);
        mock_chip8.process_5_command(0, 1);
        assert_eq!(mock_chip8.program_counter, 0x0200 + 4);
    }

    /// 0x5XY0 - Test not skipping instruction if V_X = V_Y
    /// Registers 0 and 1 set equal to not be each other, program counter will increment by 4
    #[test]
    fn test_5xy0_dont_skip() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0xFF);
        mock_chip8.cpu_registers[1] = Wrapping(0xFE);
        assert_eq!(mock_chip8.program_counter, 0x0200);
        mock_chip8.process_5_command(0, 1);
        assert_eq!(mock_chip8.program_counter, 0x0200 + 2);
    }
    
    /// 0x6XNN - Test setting VX - NN 
    #[test]
    fn test_6xnn() {
        let mut mock_chip8 = get_chip_8(None);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0));
        mock_chip8.process_6_command(0, 0xFF);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0xFF));
    }

    /// 0x7NN - Test adding NN to VX
    #[test]
    fn test_7xnn() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(2);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x02));
        mock_chip8.process_7_command(0, 0x02);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x04));
    }

    /// 0x8XY0 - Sets VX to the value of VY
    #[test]
    fn test_8xy0() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0x01);
        mock_chip8.cpu_registers[1] = Wrapping(0x02);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x01));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x02));
        mock_chip8.process_8_command(0x0000, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x02));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x02));
    }

    /// 0x8XY1 - Sets VX to the value of XX bitwise OR VY
    #[test]
    fn test_8xy1() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0xF0);
        mock_chip8.cpu_registers[1] = Wrapping(0x0F);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0xF0));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x0F));
        mock_chip8.process_8_command(0x0001, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0xFF));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x0F));
    }

    /// 0x8XY2 - Sets VX to the value of XX bitwise AND VY
    #[test]
    fn test_8xy2() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0xF0);
        mock_chip8.cpu_registers[1] = Wrapping(0x0F);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0xF0));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x0F));
        mock_chip8.process_8_command(0x0002, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x00));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x0F));
    }

    /// 0x8XY3 - Sets VX to the value of XX bitwise XOR VY
    #[test]
    fn test_8xy3() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0xF4);
        mock_chip8.cpu_registers[1] = Wrapping(0x0F);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0xF4));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x0F));
        mock_chip8.process_8_command(0x0003, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0xFB));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x0F));
    }

    /// 0x8XY4 - Adds VY to VX. VF set to 0 when borrow, 1 when there isn't
    #[test]
    fn test_8xy4() {
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0xFF);
        mock_chip8.cpu_registers[1] = Wrapping(0x02);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0xFF));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x02));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0));
        mock_chip8.process_8_command(0x0004, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x01));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x02));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(1));
    }

    /// 0x8XY5 - Subtracts VY from VX. VF set to 0 when borrow, 1 when there isn't
    #[test]
    fn test_8xy5() {
        let mut mock_chip8 = get_chip_8(None);

        // Borrow, VF should be 0
        mock_chip8.cpu_registers[0] = Wrapping(0x00);
        mock_chip8.cpu_registers[1] = Wrapping(0x01);
        mock_chip8.cpu_registers[0xF] = Wrapping(1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x00));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x01));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(1));
        mock_chip8.process_8_command(0x0005, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0xFF));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x01));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0));

        // No borrow, VF should be 1
        mock_chip8.cpu_registers[0] = Wrapping(0x01);
        mock_chip8.cpu_registers[1] = Wrapping(0x01);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x01));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x01));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0));
        mock_chip8.process_8_command(0x0005, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x00));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x01));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(1));
    }

    /// 0x8XY6 - Stores least significant bit of VX in VF and shifts VX to the right by 1
    #[test]
    fn test_8xy6() {
        let mut mock_chip8 = get_chip_8(None);
        // Least significant bit should be 1
        mock_chip8.cpu_registers[0] = Wrapping(0x0F);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(15));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0));
        mock_chip8.process_8_command(0x0006, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(7));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0b1));

        // Least significant bit should be 0
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0x0E);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(14));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0));
        mock_chip8.process_8_command(0x0006, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(7));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0b0));
    }

    /// 0x8XY7 - Subtracts VX from VY, stores in VX. VF set to 0 when borrow, 1 when there isn't
    #[test]
    fn test_8xy7() {
        let mut mock_chip8 = get_chip_8(None);

        // Borrow, VF should be 0
        mock_chip8.cpu_registers[0] = Wrapping(0x01);
        mock_chip8.cpu_registers[1] = Wrapping(0x00);
        mock_chip8.cpu_registers[0xF] = Wrapping(1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x01));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x00));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(1));
        mock_chip8.process_8_command(0x0007, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0xFF));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x00));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0));

        // No borrow, VF should be 1
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0x01);
        mock_chip8.cpu_registers[1] = Wrapping(0x0A);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x01));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x0A));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0));
        mock_chip8.process_8_command(0x0007, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x09));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x0A));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(1));
    }

    /// 0x8XYE - Stores most significant bit of VX in VF and shifts VX to left by 1
    #[test]
    fn test_8xye() {
        let mut mock_chip8 = get_chip_8(None);
        // Least significant bit should be 1
        mock_chip8.cpu_registers[0] = Wrapping(0xFF);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(255));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0));
        mock_chip8.process_8_command(0x000E, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(254));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0b1));

        // Least significant bit should be 0
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0x7F);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(127));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0));
        mock_chip8.process_8_command(0x000E, 0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(254));
        assert_eq!(mock_chip8.cpu_registers[0xF], Wrapping(0b0));
    }

    /// 9XY0 - Skips the next instruction if VX doesn't equal VY
    #[test]
    fn test_9xy0() {
        let mut mock_chip8 = get_chip_8(None);
        // Skip next instruction - Program counter increments by 4
        mock_chip8.cpu_registers[0] = Wrapping(0x0);
        mock_chip8.cpu_registers[1] = Wrapping(0x1);
        assert_ne!(mock_chip8.cpu_registers[0], mock_chip8.cpu_registers[1]);
        assert_eq!(mock_chip8.program_counter, 0x200);
        mock_chip8.process_9_command(0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x0));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x1));
        assert_eq!(mock_chip8.program_counter, 0x200 + 4);

        // Do not skip next instruction - Program counter increments by 2
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0x0);
        mock_chip8.cpu_registers[1] = Wrapping(0x0);
        assert_eq!(mock_chip8.cpu_registers[0], mock_chip8.cpu_registers[1]);
        assert_eq!(mock_chip8.program_counter, 0x200);
        mock_chip8.process_9_command(0, 1);
        assert_eq!(mock_chip8.cpu_registers[0], Wrapping(0x0));
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0x0));
        assert_eq!(mock_chip8.program_counter, 0x200 + 2);
    }

    /// ANNN - Sets index register (I) to address NNN
    #[test]
    fn test_annn() {
        let mut mock_chip8 = get_chip_8(None);
        assert_eq!(mock_chip8.index_register, Wrapping(0x0));
        mock_chip8.process_a_command(0x045F);
        assert_eq!(mock_chip8.index_register, Wrapping(0x045F));
    }

    /// BNNN - Sets program counter to address NNN plus V0
    #[test]
    fn test_bnnn() {
        // V0 is default (0)
        let mut mock_chip8 = get_chip_8(None);
        assert_eq!(mock_chip8.program_counter, 0x200);
        mock_chip8.process_b_command(0x0111);
        assert_eq!(mock_chip8.program_counter, 0x0111);

        // Set V0 to value
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.cpu_registers[0] = Wrapping(0x20);
        assert_eq!(mock_chip8.program_counter, 0x200);
        mock_chip8.process_b_command(0x0111);
        assert_eq!(mock_chip8.program_counter, 0x0131);
    }

    /// EX - Test skips on key pressed/not pressed
    #[test]
    fn test_ex() {
        // Test skip if key is pressed
        let mut mock_chip8 = get_chip_8(None);
        mock_chip8.set_keys(vec![Keycode::Q]);
        mock_chip8.cpu_registers[0] = Wrapping(4);
        assert_eq!(mock_chip8.keys[4], 1);
        assert_eq!(mock_chip8.program_counter, 0x200);
        mock_chip8.process_ex9e_command(0);
        assert_eq!(mock_chip8.program_counter, 0x200 + 4);

        // Test skip if key is not pressed - program counter increments by 2
        mock_chip8.program_counter = 0x200;
        assert_eq!(mock_chip8.keys[0], 0);
        assert_eq!(mock_chip8.cpu_registers[1], Wrapping(0));
        assert_eq!(mock_chip8.keys[4], 1);
        assert_eq!(mock_chip8.program_counter, 0x200);
        mock_chip8.process_ex9e_command(1);
        assert_eq!(mock_chip8.program_counter, 0x200 + 2);
    }
}