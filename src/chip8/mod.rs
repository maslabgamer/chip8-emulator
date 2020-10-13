use std::num::Wrapping;
use device_query::Keycode;
use rand::Rng;

pub(crate) struct Chip8 {
    memory: [u8; 4096],
    // V
    cpu_registers: [Wrapping<u8>; 16],
    // I
    index_register: u16,
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

// Mask used to remove operator from front of opcode
const OPCODE_VALUE_MASK: u16 = 0x0FFF;

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
            index_register: 0,
            program_counter: 0x200,
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            stack_pointer: 0,
            keys: [0; 16],
            draw_flag: false,
            gfx: [0; 64 * 32],
        };

        // Load fontset here
        for i in 0..80 {
            new_chip8.memory[i] = CHIP8_FONTSET[i];
        }

        new_chip8
    }

    pub fn emulate_cycle(&mut self) {
        // Fetch Opcode
        let opcode: u16 = (self.memory[self.program_counter as usize] as u16) << 8
            | (self.memory[self.program_counter as usize + 1] as u16);

        let v_x: usize = ((opcode & 0x0F00) >> 8) as usize;
        let v_y: usize = ((opcode & 0x00F0) >> 4) as usize;

        // Decode and Execute Opcode
        // Note: "NNN" denotes last three "nibbles" of two-byte opcode
        // "NN" denotes last two "nibbles" of two-byte opcode
        match opcode {
            // Calls machine code at address NNN
            0x0000..=0x0FFF => {
                match opcode {
                    0x00E0 => {
                        // clear the screen
                        println!("Clear screen");
                    }
                    0x00EE => {
                        // return from subroutine
                        self.program_counter = self.stack[self.stack_pointer as usize] + 2;
                        self.stack_pointer -= 1;
                    }
                    _ => panic!("Unknown opcode: {:#X}", opcode),
                }

                self.program_counter += 2;
            }
            // Jump to address NNN
            0x1000..=0x1FFF => {
                self.program_counter = opcode & OPCODE_VALUE_MASK;
            }
            // Calls subroutine at address NNN
            0x2000..=0x2FFF => {
                // Store current position of program counter on the stack
                self.stack_pointer += 1;
                self.stack[self.stack_pointer as usize] = self.program_counter;
                // Mask the opcode to get the address for the subroutine instruction
                self.program_counter = opcode & OPCODE_VALUE_MASK;
            }
            // Skip next instruction if VX equals NN
            0x3000..=0x3FFF => {
                let nn = (opcode & 0x00FF) as u8;
                self.program_counter += match self.cpu_registers[v_x].0 == nn {
                    true => 4,
                    false => 2
                };
            }
            // Skip next instruction if VX does NOT equals NN
            0x4000..=0x4FFF => {
                let nn = (opcode & 0x00FF) as u8;
                self.program_counter += match self.cpu_registers[v_x].0 != nn {
                    true => 4,
                    false => 2
                };
            }
            // Skip next instruction if VX equals VY
            0x5000..=0x5FFF => {
                match opcode & 0x000F {
                    0x0000 => {
                        self.program_counter += match self.cpu_registers[v_x] == self.cpu_registers[v_y] {
                            true => 4,
                            false => 2
                        };
                    }
                    _ => panic!("Unknown opcode: {:#X}", opcode),
                }
            }
            // Sets VX to NN
            0x6000..=0x6FFF => {
                self.cpu_registers[v_x] = Wrapping((opcode & 0x00FF) as u8);
                self.program_counter += 2;
            }
            // Adds NN to VX (carry flag not updated)
            0x7000..=0x7FFF => {
                self.cpu_registers[v_x] += Wrapping((opcode & 0x00FF) as u8);
                self.program_counter += 2;
            }
            // Arithmetic operators
            0x8000..=0x8FFF => {
                match opcode & 0x000F {
                    // 0x8XY0 - Sets VX to the value of VY
                    0x0000 => {
                        self.cpu_registers[v_x] = self.cpu_registers[v_y];
                        self.program_counter += 2;
                    },
                    // 0x8XY1 - Sets VX to bitwise OR operation of VX and VY
                    0x0001 => {
                        self.cpu_registers[v_x] |= self.cpu_registers[v_y];
                        self.program_counter += 2;
                    }
                    // 0x8XY2 - Sets VX to bitwise AND operation of VX and VY
                    0x0002 => {
                        self.cpu_registers[v_x] &= self.cpu_registers[v_y];
                        self.program_counter += 2;
                    },
                    // 0x8XY3 - Sets VX to bitwise XOR operation of VX and VY
                    0x0003 => {
                        self.cpu_registers[v_x] ^= self.cpu_registers[v_y];
                        self.program_counter += 2;
                    },
                    // 0x8XY4 - Adds value of VY to VX
                    0x0004 => {
                        self.cpu_registers[0xF] = Wrapping(match self.cpu_registers[v_x].0 > (0xFF - self.cpu_registers[v_y].0) {
                            true => 1, // carry
                            false => 0
                        });

                        self.cpu_registers[v_x] += self.cpu_registers[v_y];
                        self.program_counter += 2;
                    },
                    // 0x8XY5 - VY is subtracted from VX. VF set to 0 when there's borrow, 1, when there isn't
                    0x0005 => {
                        self.cpu_registers[0xF] = Wrapping(match self.cpu_registers[v_y] > self.cpu_registers[v_x] {
                            true => 0x00, // Borrow occurred
                            false => 0x01
                        });
                        self.cpu_registers[v_x] -= self.cpu_registers[v_y];
                        self.program_counter += 2;
                    },
                    _ => panic!("Unknown opcode: {:#X}", opcode),
                }
            }
            // Sets index register to value NNN
            0xA000..=0xAFFF => {
                let value = opcode & OPCODE_VALUE_MASK;
                self.index_register = value;
                self.program_counter += 2;
            }
            0xC000..=0xCFFF => {
                let nn = (opcode & 0x00FF) as u8;
                self.cpu_registers[v_x] = Wrapping(rand::thread_rng().gen_range(0x00, 0xFF) & nn);
                self.program_counter += 2;
            }
            // Draw sprite at coordinate (VX, VY) 8 pixels wide and N pixels high where N is last nibble
            0xD000..=0xDFFF => {
                // Fetch position and height of sprite
                let x = self.cpu_registers[v_x].0 as u16;
                let y = self.cpu_registers[v_y].0 as u16;
                // Pixel value
                let height: u16 = opcode & 0x000F;

                // Reset register VF
                self.cpu_registers[0xF] = Wrapping(0);
                for y_line in 0..height {
                    // fetch pixel value from memory starting at location I
                    let pixel = self.memory[(self.index_register + y_line) as usize];
                    // Sprite is always 8 wide, loop over 8 bits to draw one row
                    for x_line in 0..8 {
                        // Check if current pixel is set to 1 (using >> x_line to scan through byte)
                        if (pixel & (0x80 >> x_line)) != 0 {
                            let gfx_idx: usize = ((x + x_line + ((y + y_line) * 64)) as usize) % self.gfx.len();

                            // If current pixel is 1 we need to set the VF register
                            if self.gfx[gfx_idx] == 1 {
                                self.cpu_registers[0xF] = Wrapping(1);
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
            }
            // Key input
            0xE000..=0xEFFF => {
                // Only really care about EX9E and EXA1
                match opcode & 0x00FF {
                    // EX9E - Skip next instruction if key stored in VX is pressed
                    0x009E => {
                        self.program_counter += match self.keys[v_x] == 1 {
                            true => 4,
                            false => 2,
                        };
                    }
                    // EXA1 - Skip next instruction if key stored in VX is NOT pressed
                    0x00A1 => {
                        let key = self.cpu_registers[v_x].0 as usize;
                        self.program_counter += match self.keys[key] != 1 {
                            true => 4,
                            false => 2,
                        };
                    }
                    _ => panic!("Unknown opcode: {:#X}", opcode),
                }
            }
            0xF000..=0xFFFF => {
                match opcode & 0xF0FF {
                    // Store current value of delay timer in register VX
                    0xF007 => {
                        self.cpu_registers[v_x] = Wrapping(self.delay_timer);
                        self.program_counter += 2;
                    },
                    // Set delay timer to value of register VX
                    0xF015 => {
                        self.delay_timer = self.cpu_registers[v_x].0;
                        self.program_counter += 2;
                    },
                    // Set sound timer to VX
                    0xF018 => {
                        self.sound_timer = self.cpu_registers[v_x].0;
                        self.program_counter += 2;
                    },
                    // Sets I to location of the sprite for character in VX
                    0xF029 => {
                        self.index_register = (self.cpu_registers[v_x].0 as u16) * 5;
                        self.program_counter += 2;
                    },
                    // Store binary-coded decimal representation of VX at addresses I, I+1, and I+2
                    0xF033 => { // opcode 0xFX33
                        self.memory[self.index_register as usize] = self.cpu_registers[v_x].0 / 100;
                        self.memory[self.index_register as usize + 1] = (self.cpu_registers[v_x].0 / 100) % 10;
                        self.memory[self.index_register as usize + 2] = self.cpu_registers[v_x].0 % 10;
                        self.program_counter += 2;
                    },
                    // Fills V0 to VX (including VX) with values from memory starting at address I
                    0xF065 => {
                        println!("Loading V0 to vx ({})", v_x);
                        for i in 0..v_x + 1 {
                            self.cpu_registers[i] = Wrapping(self.memory[self.index_register as usize + i]);
                        }
                        self.program_counter += 2;
                    },
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

    pub fn draw_to_buffer(&mut self, buffer: &mut Vec<u32>) {
        if self.draw_flag {
            for pixel_idx in 0..buffer.len() {
                buffer[pixel_idx] = if self.gfx[pixel_idx] == 0 {
                    0x0000
                } else {
                    0x0FFF
                };
            }
        }
        self.draw_flag = false;
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
