use std::io::Seek;
use std::io::{Read, SeekFrom};
use std::error::Error;
use std::fs::File;
use std::{env, format};
use byteorder::{ByteOrder, LittleEndian};


// The range of bytes from the beginning of the file where the VM looks for the begin opcode
const BEGIN_OPCODE_SEARCH_RANGE: usize = 4096;
// The length of each stack in the program
const BUFFER_LENGTH: usize = 4096;
// Length for the memory stack
// This is equal to the buffer for now, but maybe it could be a lot larger later, if needed.
const MEMORY_STACK_LENGTH: usize = BUFFER_LENGTH;




#[derive(Debug, Copy, Clone)]
enum Opcode {
    _Begin,             // Bytecode start, denoted with 'b', discarded when read
    FlushAndRestart,    // Return the result of the current expression and evaluate next expression
    End,                // Bytecode end, 'e', again, discarded when read
    Add,
    Subtract,
    IsEqual,
    IsGreater,
    Invert,
    Skip,
    StoreAtMemoryIndex(usize),
    Unsupported(u8),
}

#[derive(Debug, Copy, Clone)]
enum Instruction {
    Opcode(Opcode),
    Number(i16),
    End,
}

enum NextInstructionStep {
    Continue,
    EndExecution,
    FlushAnswerAndProceed,
    StoreAnswerAndProceed(usize),
}

// enum OperationState {
//     FlushAndRestart,
//     End,
// }

fn main() -> Result<(), Box<dyn Error>> {
    // Command Line Arguments
    let args: Vec<String> = env::args().collect();

    // Initialising the vm
    let bytecode_path = &args[1];
    let mut operand_stack = [0i16; BUFFER_LENGTH];
    let mut input_stack;
    let mut top;
    let mut file_pointer = match find_start(bytecode_path) {
        Ok(Some(t)) => t,
        Ok(None) => panic!("{:?}", "Opcode::Begin not found."),
        Err(e) => panic!("{:?}", e),
    };
    let mut memory_stack = [0i16; MEMORY_STACK_LENGTH];

    // All of these stacks could be extracted into structs, and then have implementations for safe indexing


    // Execution loop
    'bytecode_read: loop {
        top = 0;
        input_stack = read_intructions(bytecode_path, &mut file_pointer)?;

        '_expression_read: for instruction in input_stack {

            match match_instruction(instruction, &mut operand_stack, &mut top) {
                NextInstructionStep::Continue => continue,
                NextInstructionStep::EndExecution => break 'bytecode_read,
                NextInstructionStep::FlushAnswerAndProceed => {
                    if top == 1 {                                           // If there's only one number in the operand stack
                        println!("Answer: {:?}", operand_stack[0]);
                    }
                    else {
                        println!("Operator-Operand mismatch. Peeking stack: {:?}", operand_stack[top - 1]);
                    }
                },
                NextInstructionStep::StoreAnswerAndProceed(memory_index) => {
                    if top == 1 {                                           // If there's only one number in the operand stack
                        memory_stack[memory_index] = operand_stack[0];
                    }
                    else {
                        println!("Operator-Operand mismatch. Storing: {:?} at index: {:?}", operand_stack[top - 1], memory_index);
                        memory_stack[memory_index] = operand_stack[top - 1];
                    }
                },
            }


            // This has been extracted into its own function, this was just wrong
            // It is still here temporarily as a reference
            //
            //
            // match i {
            //     Instruction::End => {
            //         break 'expression_read;
            //     },
            //     Instruction::Number(t) => {
            //         operand_stack[top] = t;
            //         top += 1;
            //     },
            //     Instruction::Opcode(t) => {
            //         match operate(t, &mut operand_stack, &mut top) {
            //             Ok(t) => {
            //                 if t == 1 || t == 2 {
            //                     if top == 1 {
            //                         println!("Answer: {:?}", operand_stack[0]);
            //                     }
            //                     else {
            //                         println!("Operator-Operand mismatch, insufficient operations.");
            //                     }

            //                     if t == 2 {
            //                         break 'bytecode_read;
            //                     }
            //                 }
            //             },
            //             Err(e) => println!("{:?}", e),
            //         }
            //     },

            // }
        }
    };


    Ok(())

}

// in the execution loop
fn match_instruction(instruction: Instruction, operand_stack: &mut [i16; BUFFER_LENGTH], top: &mut usize) -> NextInstructionStep {
    match instruction {
                Instruction::End => {
                    println!("Execution stopped, no 'End' or 'Flush' encountered.");
                    return NextInstructionStep::EndExecution;
                },
                Instruction::Number(t) => {
                    operand_stack[*top] = t;
                    *top += 1;
                    NextInstructionStep::Continue
                },
                Instruction::Opcode(t) => {
                    match operate(t, operand_stack, top) {
                        Ok(t) => t,
                        Err(e) => {
                            println!("{:?}", e);
                            NextInstructionStep::Continue
                        },
                    }
                },

            }
}

fn find_start(bytecode_path: &str) -> Result<Option<usize>, Box<dyn Error>> {
    let mut file = File::open(bytecode_path)?;

    // Limited search range for now, probably should be changed to look through a whole file in chunks
    // I don't know if that will be a problem if the program attempts to read through a very large file
    let mut buffer = [0u8; BEGIN_OPCODE_SEARCH_RANGE];

    let bytes_read = file.read(&mut buffer)?;

    for i in 0..bytes_read {
        if buffer[i] == b'B' {
            return Ok(Some((i + 1).try_into()?));
        }
    }

    Ok(None)
}

fn read_intructions(bytecode_path: &str, file_pointer: &mut usize) -> Result<[Instruction; BUFFER_LENGTH], Box<dyn Error>>{
    let mut file = File::open(bytecode_path)?;
    file.seek(SeekFrom::Start(*file_pointer as u64))?;

    let mut buffer =  [0u8; BUFFER_LENGTH];
    let bytes_read = file.read(&mut buffer)?;

    let mut instructions = [Instruction::End; BUFFER_LENGTH];
    let mut instruction_index = 0;
    let mut buffer_index = 0;

    while buffer_index < bytes_read {

        // Opcode::End
        if buffer[buffer_index] == b'E' {
            instructions[instruction_index] = Instruction::Opcode(Opcode::End);
            *file_pointer += buffer_index + 1;
            *file_pointer += 1;
            return Ok(instructions);
        }

        // Opcode::FlushAndRestart
        else if buffer[buffer_index] == b'F' {
            instructions[instruction_index] = Instruction::Opcode(Opcode::FlushAndRestart);
            *file_pointer += 1;
            return Ok(instructions);
        }

        else {

            instructions[instruction_index] = match buffer[buffer_index] {
                b'A' => Instruction::Opcode(Opcode::Add),
                b'S' => Instruction::Opcode(Opcode::Subtract),
                b'Q' => Instruction::Opcode(Opcode::IsEqual),
                b'G' => Instruction::Opcode(Opcode::IsGreater),
                b'I' => Instruction::Opcode(Opcode::Invert),
                b' ' | b'\n' => Instruction::Opcode(Opcode::Skip),
                b'N' => {
                    let num = [buffer[buffer_index+1], buffer[buffer_index+2]];
                    buffer_index += 2;
                    *file_pointer += 1;
                    let decoded_num = LittleEndian::read_i16(&num);
                    println!("{:?}", decoded_num);
                    Instruction::Number(decoded_num)
                }
                b'M' => {
                    let num = [buffer[buffer_index+1], buffer[buffer_index+2]];
                    buffer_index += 2;
                    *file_pointer += 1;
                    let decoded_index = LittleEndian::read_i16(&num);
                    println!("{:?}", decoded_index);
                    Instruction::Opcode(Opcode::StoreAtMemoryIndex(decoded_index as usize % MEMORY_STACK_LENGTH))
                }
                t => Instruction::Opcode(Opcode::Unsupported(t)),
            }
        }
        instruction_index += 1;
        buffer_index += 1;
        *file_pointer += 1;
    }

    Err("NoEndOrFlush".into())


}

fn operate(opcode: Opcode, operand_stack: &mut [i16; BUFFER_LENGTH], top: &mut usize) -> Result<NextInstructionStep, String>{
    match opcode {
        Opcode::_Begin => Err("This should never be called, it just exists for clarity.".to_owned()),

        Opcode::FlushAndRestart => {
            Ok(NextInstructionStep::FlushAnswerAndProceed)           // Replace with NextInstructionStep
        },

        Opcode::End => {
            Ok(NextInstructionStep::EndExecution)           // Replace with NextInstructionStep
        },

        Opcode::Add => {
            operand_stack[*top - 2] += operand_stack[*top - 1];
            operand_stack[*top - 1] = 0;
            *top -= 1;
            Ok(NextInstructionStep::Continue)
        },

        Opcode::Subtract => {
            operand_stack[*top - 2] -= operand_stack[*top - 1];
            operand_stack[*top - 1] = 0;
            *top -= 1;
            Ok(NextInstructionStep::Continue)
        },

        Opcode::IsEqual => {
            operand_stack[*top - 2] = (operand_stack[*top - 2] == operand_stack[*top - 1]) as i16;
            operand_stack[*top - 1] = 0;
            *top -= 1;
            Ok(NextInstructionStep::Continue)
        },

        Opcode::IsGreater => {
            operand_stack[*top - 2] = (operand_stack[*top - 2] > operand_stack[*top - 1]) as i16;
            operand_stack[*top - 1] = 0;
            *top -= 1;
            Ok(NextInstructionStep::Continue)
        },

        Opcode::Invert => {
            // ! returns the negative of a number when called with an i16
            // So used `== 0` instead
            operand_stack[*top - 1] = (operand_stack[*top - 1] == 0) as i16;
            Ok(NextInstructionStep::Continue)
        },

        Opcode::Skip => Ok(NextInstructionStep::Continue),

        Opcode::Unsupported(t) => Err(format!("Unrecognised Opcode: {}", t).to_owned()),

        Opcode::StoreAtMemoryIndex(index) => Ok(NextInstructionStep::StoreAnswerAndProceed(index)),
    }
}
