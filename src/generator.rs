//! IR generators for verifier/JIT stress.

use rand::Rng;

use crate::ir::{
    AliasClass, AluOp, BasicRegion, Cond, FakeDepStrategy, IR, IrInstr, Reg, StackPressureStrategy,
    SyscallId, Value,
};
use crate::validate::validate;

/// Generate verifier-focused stress IR.
pub fn gen_verifier_stress() -> IR {
    IR {
        regions: vec![
            BasicRegion {
                label: "entry".to_string(),
                instructions: vec![IrInstr::Br {
                    cond: Cond::Gtu,
                    lhs: Reg::R2,
                    rhs: Value::Imm(16),
                    target: 1,
                }],
            },
            BasicRegion {
                label: "true".to_string(),
                instructions: vec![
                    IrInstr::Alu {
                        dst: Reg::R1,
                        op: AluOp::Add,
                        src: Value::Imm(8),
                    },
                    IrInstr::StackPressure {
                        bytes: 64,
                        strategy: StackPressureStrategy::DeadAlloc,
                    },
                    IrInstr::BrUncond { target: 3 },
                ],
            },
            BasicRegion {
                label: "false".to_string(),
                instructions: vec![
                    IrInstr::Alu {
                        dst: Reg::R1,
                        op: AluOp::Add,
                        src: Value::Imm(4),
                    },
                    IrInstr::StackPressure {
                        bytes: 96,
                        strategy: StackPressureStrategy::DeadAlloc,
                    },
                    IrInstr::BrUncond { target: 3 },
                ],
            },
            BasicRegion {
                label: "merge".to_string(),
                instructions: vec![
                    IrInstr::AliasProbe {
                        ptr: Reg::R6,
                        alias_class: AliasClass::CrossRegion,
                    },
                    IrInstr::BrUncond { target: 4 },
                ],
            },
            BasicRegion {
                label: "exit".to_string(),
                instructions: vec![IrInstr::Return],
            },
        ],
        entry_region: 0,
    }
}

/// Generate JIT-focused stress IR.
pub fn gen_jit_stress() -> IR {
    let regs = [
        Reg::R0,
        Reg::R1,
        Reg::R2,
        Reg::R3,
        Reg::R4,
        Reg::R5,
        Reg::R6,
        Reg::R7,
        Reg::R8,
        Reg::R9,
    ];

    let mut instructions = Vec::new();
    for (idx, reg) in regs.iter().copied().enumerate() {
        instructions.push(IrInstr::Mov {
            dst: reg,
            src: Value::Imm(idx as i64),
        });
    }

    for i in 0..45usize {
        let reg = regs[i % regs.len()];
        instructions.push(IrInstr::Alu {
            dst: reg,
            op: AluOp::Add,
            src: Value::Imm((i as i64 % 7) + 1),
        });
        instructions.push(IrInstr::FakeDep {
            reg,
            strategy: FakeDepStrategy::AddSubPair,
        });
        instructions.push(IrInstr::FakeDep {
            reg,
            strategy: FakeDepStrategy::MovSelf,
        });
    }

    instructions.push(IrInstr::Syscall {
        id: SyscallId::SolLog,
        args: [
            Some(Value::Register(Reg::R1)),
            Some(Value::Register(Reg::R2)),
            Some(Value::Register(Reg::R3)),
            Some(Value::Register(Reg::R4)),
            Some(Value::Register(Reg::R5)),
        ],
    });
    instructions.push(IrInstr::Return);

    IR {
        regions: vec![BasicRegion {
            label: "jit_straightline".to_string(),
            instructions,
        }],
        entry_region: 0,
    }
}

fn random_writable_reg(rng: &mut impl Rng) -> Reg {
    match rng.gen_range(0..10) {
        0 => Reg::R0,
        1 => Reg::R1,
        2 => Reg::R2,
        3 => Reg::R3,
        4 => Reg::R4,
        5 => Reg::R5,
        6 => Reg::R6,
        7 => Reg::R7,
        8 => Reg::R8,
        _ => Reg::R9,
    }
}

fn random_alias(rng: &mut impl Rng) -> AliasClass {
    match rng.gen_range(0..3) {
        0 => AliasClass::CrossRegion,
        1 => AliasClass::StackOverlap {
            offset_a: -8,
            offset_b: -16,
        },
        _ => AliasClass::InputRegion {
            offset_a: 8,
            offset_b: 24,
        },
    }
}

fn mutate_once(seed: &IR, rng: &mut impl Rng) -> IR {
    let mut mutant = seed.clone();
    if mutant.regions.is_empty() {
        return seed.clone();
    }

    let kind = rng.gen_range(0..6);
    match kind {
        0 => {
            let ridx = rng.gen_range(0..mutant.regions.len());
            let pos = rng.gen_range(0..=mutant.regions[ridx].instructions.len());
            let strategy = match rng.gen_range(0..3) {
                0 => FakeDepStrategy::XorZero,
                1 => FakeDepStrategy::MovSelf,
                _ => FakeDepStrategy::AddSubPair,
            };
            mutant.regions[ridx].instructions.insert(
                pos,
                IrInstr::FakeDep {
                    reg: random_writable_reg(rng),
                    strategy,
                },
            );
        }
        1 => {
            let ridx = rng.gen_range(0..mutant.regions.len());
            let pos = rng.gen_range(0..=mutant.regions[ridx].instructions.len());
            let strategy = if rng.gen_bool(0.8) {
                StackPressureStrategy::DeadAlloc
            } else {
                StackPressureStrategy::SpillReload {
                    reg: random_writable_reg(rng),
                }
            };
            mutant.regions[ridx].instructions.insert(
                pos,
                IrInstr::StackPressure {
                    bytes: rng.gen_range(8..=256),
                    strategy,
                },
            );
        }
        2 => {
            let ridx = rng.gen_range(0..mutant.regions.len());
            let pos = rng.gen_range(0..=mutant.regions[ridx].instructions.len());
            mutant.regions[ridx].instructions.insert(
                pos,
                IrInstr::AliasProbe {
                    ptr: random_writable_reg(rng),
                    alias_class: random_alias(rng),
                },
            );
        }
        3 => {
            let ridx = rng.gen_range(0..mutant.regions.len());
            let len = mutant.regions[ridx].instructions.len();
            if len >= 2 {
                let i = rng.gen_range(0..len);
                let mut j = rng.gen_range(0..len);
                if i == j {
                    j = (j + 1) % len;
                }
                mutant.regions[ridx].instructions.swap(i, j);
            }
        }
        4 => {
            if mutant.regions.len() >= 2 {
                let copy_idx = rng.gen_range(0..mutant.regions.len());
                let mut new_region = mutant.regions[copy_idx].clone();
                new_region.label = format!("{}_copy", new_region.label);
                let new_index = mutant.regions.len();
                mutant.regions.push(new_region);
                for region in &mut mutant.regions {
                    for instr in &mut region.instructions {
                        match instr {
                            IrInstr::Br { target, .. }
                            | IrInstr::BrUncond { target }
                            | IrInstr::Call { target } => {
                                if rng.gen_bool(0.1) {
                                    *target = new_index;
                                    return mutant;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        _ => {
            let boundary = [0_i64, 1, -1, i64::MAX, i64::MIN];
            for region in &mut mutant.regions {
                for instr in &mut region.instructions {
                    match instr {
                        IrInstr::Alu {
                            src: Value::Imm(value),
                            ..
                        }
                        | IrInstr::Mov {
                            src: Value::Imm(value),
                            ..
                        }
                        | IrInstr::Store {
                            src: Value::Imm(value),
                            ..
                        } => {
                            *value = boundary[rng.gen_range(0..boundary.len())];
                            return mutant;
                        }
                        IrInstr::Br {
                            rhs: Value::Imm(value),
                            ..
                        } => {
                            *value = boundary[rng.gen_range(0..boundary.len())];
                            return mutant;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    mutant
}

/// Generate a mutated IR from a seed program.
pub fn gen_random_mutant(seed: &IR, rng: &mut impl Rng) -> IR {
    for _ in 0..10 {
        let mutant = mutate_once(seed, rng);
        if validate(&mutant).is_ok() {
            return mutant;
        }
    }
    seed.clone()
}
