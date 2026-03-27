use sbpf_tool::ir::{AluOp, BasicRegion, IR, IrInstr, Reg, Value};
use sbpf_tool::lowering::{lower, LoweringConfig, StressMode};
use solana_sbpf::ebpf;

fn reg_from_index(idx: u8) -> Result<Reg, String> {
    match idx {
        0 => Ok(Reg::R0),
        1 => Ok(Reg::R1),
        2 => Ok(Reg::R2),
        3 => Ok(Reg::R3),
        4 => Ok(Reg::R4),
        5 => Ok(Reg::R5),
        6 => Ok(Reg::R6),
        7 => Ok(Reg::R7),
        8 => Ok(Reg::R8),
        9 => Ok(Reg::R9),
        10 => Ok(Reg::FP),
        _ => Err(format!("invalid register index: {idx}")),
    }
}

/// Demo-only lifting for a tiny opcode subset used below.
fn lift_demo(program: &[u8]) -> Result<IR, String> {
    if program.len() % ebpf::INSN_SIZE != 0 {
        return Err("program length must be a multiple of INSN_SIZE".to_string());
    }

    let mut instructions = Vec::new();
    let count = program.len() / ebpf::INSN_SIZE;
    for pc in 0..count {
        let insn = ebpf::get_insn(program, pc);
        match insn.opc {
            ebpf::MOV64_IMM => instructions.push(IrInstr::Mov {
                dst: reg_from_index(insn.dst)?,
                src: Value::Imm(insn.imm),
            }),
            ebpf::ADD64_IMM => instructions.push(IrInstr::Alu {
                dst: reg_from_index(insn.dst)?,
                op: AluOp::Add,
                src: Value::Imm(insn.imm),
            }),
            ebpf::EXIT => instructions.push(IrInstr::Return),
            opc => {
                return Err(format!(
                    "demo lift does not support opcode 0x{opc:02x} at pc={pc}"
                ))
            }
        }
    }

    Ok(IR {
        regions: vec![BasicRegion {
            label: "entry".to_string(),
            instructions,
        }],
        entry_region: 0,
    })
}

fn main() -> Result<(), String> {
    // sBPF program:
    //   mov64 r0, 5
    //   add64 r0, 7
    //   exit
    let original_program: Vec<u8> = vec![
        ebpf::Insn {
            ptr: 0,
            opc: ebpf::MOV64_IMM,
            dst: 0,
            src: 0,
            off: 0,
            imm: 5,
        }
        .to_vec(),
        ebpf::Insn {
            ptr: 1,
            opc: ebpf::ADD64_IMM,
            dst: 0,
            src: 0,
            off: 0,
            imm: 7,
        }
        .to_vec(),
        ebpf::Insn {
            ptr: 2,
            opc: ebpf::EXIT,
            dst: 0,
            src: 0,
            off: 0,
            imm: 0,
        }
        .to_vec(),
    ]
    .concat();

    let ir = lift_demo(&original_program)?;
    println!("Lifted IR:\n{ir:#?}\n");

    let lowered = lower(
        &ir,
        &LoweringConfig {
            mode: StressMode::Both,
            max_stack_depth: 4096,
            max_insn_count: 256,
        },
    )
    .map_err(|e| format!("lowering failed: {e:?}"))?;

    println!("Original bytes : {:02x?}", original_program);
    println!("Lowered  bytes : {:02x?}", lowered);

    if lowered == original_program {
        println!("\nRound-trip success: lowered bytes are identical.");
    } else {
        println!("\nRound-trip mismatch.");
    }

    Ok(())
}
