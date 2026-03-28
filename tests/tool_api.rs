use rand::SeedableRng;
use sbpf_tool::generator::{gen_complex_stress, gen_jit_stress, gen_random_mutant, gen_verifier_stress};
use sbpf_tool::ir::{AliasClass, BasicRegion, IR, IrInstr, Reg, StackPressureStrategy, Value};
use sbpf_tool::lowering::{lower, LoweringConfig, LoweringError, StressMode};
use sbpf_tool::validate::validate;

fn simple_ir() -> IR {
    IR {
        regions: vec![BasicRegion {
            label: "entry".to_string(),
            instructions: vec![IrInstr::Mov {
                dst: Reg::R0,
                src: Value::Imm(1),
            }],
        }],
        entry_region: 0,
    }
}

#[test]
fn validate_accepts_minimal_returning_program() {
    let mut ir = simple_ir();
    ir.regions[0].instructions.push(IrInstr::Return);
    assert!(validate(&ir).is_ok());
}

#[test]
fn lowering_reports_stack_overflow() {
    let ir = IR {
        regions: vec![BasicRegion {
            label: "entry".to_string(),
            instructions: vec![
                IrInstr::StackPressure {
                    bytes: 8192,
                    strategy: StackPressureStrategy::DeadAlloc,
                },
                IrInstr::Return,
            ],
        }],
        entry_region: 0,
    };

    let result = lower(
        &ir,
        &LoweringConfig {
            mode: StressMode::Both,
            max_stack_depth: 256,
            max_insn_count: 10_000,
        },
    );
    assert!(matches!(result, Err(LoweringError::StackOverflow { .. })));
}

#[test]
fn generators_produce_valid_ir() {
    let verifier = gen_verifier_stress();
    let jit = gen_jit_stress();

    assert!(validate(&verifier).is_ok());
    assert!(validate(&jit).is_ok());
}

#[test]
fn mutant_preserves_validity() {
    let seed = gen_verifier_stress();
    let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
    let mutant = gen_random_mutant(&seed, &mut rng);
    assert!(validate(&mutant).is_ok());
}

#[test]
fn lowering_expands_alias_probe() {
    let ir = IR {
        regions: vec![BasicRegion {
            label: "entry".to_string(),
            instructions: vec![
                IrInstr::AliasProbe {
                    ptr: Reg::R6,
                    alias_class: AliasClass::CrossRegion,
                },
                IrInstr::Return,
            ],
        }],
        entry_region: 0,
    };

    let bytes = lower(
        &ir,
        &LoweringConfig {
            mode: StressMode::Verifier,
            max_stack_depth: 4096,
            max_insn_count: 10_000,
        },
    )
    .expect("lower should succeed");
    assert!(bytes.len() >= 4 * 8);
}

#[test]
fn complex_generator_creates_large_lowerable_program() {
    let mut rng = rand::rngs::SmallRng::seed_from_u64(7);
    let ir = gen_complex_stress(&mut rng, 12, 24);

    assert!(validate(&ir).is_ok());
    assert!(ir.regions.len() >= 12);
    let total_ir_insns: usize = ir.regions.iter().map(|r| r.instructions.len()).sum();
    assert!(total_ir_insns >= 200);

    let bytes = lower(
        &ir,
        &LoweringConfig {
            mode: StressMode::Both,
            max_stack_depth: 4096,
            max_insn_count: 50_000,
        },
    )
    .expect("complex IR should lower");

    assert!(bytes.len() / 8 >= 250);
}

#[test]
fn complex_generator_respects_semantic_constraints() {
    let mut rng = rand::rngs::SmallRng::seed_from_u64(11);
    let ir = gen_complex_stress(&mut rng, 10, 20);

    for region in &ir.regions {
        for instr in &region.instructions {
            match instr {
                IrInstr::Alu { dst, .. } | IrInstr::Mov { dst, .. } | IrInstr::Load { dst, .. } => {
                    assert_ne!(*dst, Reg::FP, "generator must not write to FP");
                }
                IrInstr::Store { base, offset, .. } => {
                    assert_eq!(*base, Reg::FP, "stores should use FP-based stack addressing");
                    assert!(*offset < 0, "stack stores should use negative FP offsets");
                    assert_eq!(*offset % 8, 0, "stack stores should be 8-byte aligned");
                }
                IrInstr::Syscall { args, .. } => {
                    let mut saw_none = false;
                    for arg in args {
                        if arg.is_none() {
                            saw_none = true;
                        } else {
                            assert!(!saw_none, "syscall args must be positional without gaps");
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
