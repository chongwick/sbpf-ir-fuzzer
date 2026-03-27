use rand::SeedableRng;
use sbpf_tool::generator::{gen_jit_stress, gen_random_mutant, gen_verifier_stress};
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
