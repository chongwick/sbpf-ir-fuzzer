use sbpf_tool::ir::{BasicRegion, IR, IrInstr, Reg, Value};
use sbpf_tool::lowering::{lower, LoweringConfig, LoweringError, StressMode};
use sbpf_tool::validate::{validate, ValidationError};
use solana_sbpf::ebpf;

fn ir_with(instr: IrInstr) -> IR {
    IR {
        regions: vec![BasicRegion {
            label: "entry".to_string(),
            instructions: vec![instr, IrInstr::Return],
        }],
        entry_region: 0,
    }
}

#[test]
fn validate_rejects_write_to_fp() {
    let ir = ir_with(IrInstr::Mov {
        dst: Reg::FP,
        src: Value::Imm(1),
    });

    let err = validate(&ir).expect_err("expected validation failure");
    assert!(matches!(
        err.first(),
        Some(ValidationError::WriteToFramePointer { .. })
    ));
}

#[test]
fn validate_rejects_bad_branch_target() {
    let ir = IR {
        regions: vec![BasicRegion {
            label: "entry".to_string(),
            instructions: vec![IrInstr::BrUncond { target: 99 }, IrInstr::Return],
        }],
        entry_region: 0,
    };

    let err = validate(&ir).expect_err("expected validation failure");
    assert!(matches!(
        err.first(),
        Some(ValidationError::BranchTargetOutOfBounds { .. })
    ));
}

#[test]
fn validate_rejects_syscall_arg_gap() {
    let ir = ir_with(IrInstr::Syscall {
        id: sbpf_tool::ir::SyscallId::SolLog,
        args: [None, Some(Value::Imm(1)), None, None, None],
    });

    let err = validate(&ir).expect_err("expected validation failure");
    assert!(matches!(err.first(), Some(ValidationError::SyscallArgGap { .. })));
}

#[test]
fn lowering_rejects_instruction_limit() {
    let ir = ir_with(IrInstr::Mov {
        dst: Reg::R0,
        src: Value::Imm(1),
    });

    let result = lower(
        &ir,
        &LoweringConfig {
            mode: StressMode::Both,
            max_stack_depth: 1024,
            max_insn_count: 1,
        },
    );

    assert!(matches!(
        result,
        Err(LoweringError::InsnLimitExceeded { .. })
    ));
}

#[test]
fn lowering_appends_exit_for_non_returning_last_region() {
    let ir = IR {
        regions: vec![BasicRegion {
            label: "entry".to_string(),
            instructions: vec![IrInstr::Mov {
                dst: Reg::R0,
                src: Value::Imm(7),
            }],
        }],
        entry_region: 0,
    };

    let bytes = lower(
        &ir,
        &LoweringConfig {
            mode: StressMode::Both,
            max_stack_depth: 1024,
            max_insn_count: 100,
        },
    )
    .expect("lower should succeed");

    assert_eq!(bytes.len(), 2 * ebpf::INSN_SIZE);
    let last = ebpf::get_insn(&bytes, 1);
    assert_eq!(last.opc, ebpf::EXIT);
}
