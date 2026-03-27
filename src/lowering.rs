//! IR to sBPF lowering.

use crate::ir::{AliasClass, AluOp, Cond, IR, IrInstr, MemSize, Reg, StackPressureStrategy, SyscallId, Value};
use crate::validate::{validate, ValidationError};
use solana_sbpf::ebpf;
use std::convert::TryFrom;

/// Lowering mode preference.
#[derive(Debug, Clone, Copy)]
pub enum StressMode {
    /// Prefer verifier-heavy patterns.
    Verifier,
    /// Prefer JIT-heavy patterns.
    Jit,
    /// Balance both stress styles.
    Both,
}

/// Lowering configuration.
#[derive(Debug, Clone, Copy)]
pub struct LoweringConfig {
    /// Stress mode.
    pub mode: StressMode,
    /// Maximum stack depth in bytes.
    pub max_stack_depth: u32,
    /// Maximum instruction count.
    pub max_insn_count: usize,
}

/// Lowering failures.
#[derive(Debug, Clone)]
pub enum LoweringError {
    /// IR failed pre-lowering validation.
    ValidationFailed {
        /// Validation failures.
        errors: Vec<ValidationError>,
    },
    /// Stack depth exceeded configured maximum.
    StackOverflow {
        /// Observed stack usage in bytes.
        depth: u32,
        /// Configured maximum.
        max: u32,
    },
    /// Instruction count exceeded configured maximum.
    InsnLimitExceeded {
        /// Observed instruction count.
        count: usize,
        /// Configured maximum.
        max: usize,
    },
    /// A branch/call target cannot be represented.
    OffsetOutOfRange,
}

fn value_is_register(value: &Value) -> bool {
    matches!(value, Value::Register(_))
}

fn build_insn(opc: u8, dst: Reg, src: Reg, off: i16, imm: i64) -> ebpf::Insn {
    ebpf::Insn {
        ptr: 0,
        opc,
        dst: dst.index(),
        src: src.index(),
        off,
        imm,
    }
}

fn emit(insns: &mut Vec<ebpf::Insn>, mut insn: ebpf::Insn) {
    insn.ptr = insns.len();
    insns.push(insn);
}

fn lower_mov(dst: Reg, src: &Value) -> ebpf::Insn {
    match src {
        Value::Imm(imm) => build_insn(ebpf::MOV64_IMM, dst, Reg::R0, 0, *imm),
        Value::Register(reg) => build_insn(ebpf::MOV64_REG, dst, *reg, 0, 0),
    }
}

fn lower_alu(dst: Reg, op: AluOp, src: &Value) -> ebpf::Insn {
    let reg = value_is_register(src);
    let opc = match (op, reg) {
        (AluOp::Add, false) => ebpf::ADD64_IMM,
        (AluOp::Add, true) => ebpf::ADD64_REG,
        (AluOp::Sub, false) => ebpf::SUB64_IMM,
        (AluOp::Sub, true) => ebpf::SUB64_REG,
        (AluOp::Mul, false) => ebpf::MUL64_IMM,
        (AluOp::Mul, true) => ebpf::MUL64_REG,
        (AluOp::Div, false) => ebpf::DIV64_IMM,
        (AluOp::Div, true) => ebpf::DIV64_REG,
        (AluOp::Mod, false) => ebpf::MOD64_IMM,
        (AluOp::Mod, true) => ebpf::MOD64_REG,
        (AluOp::Or, false) => ebpf::OR64_IMM,
        (AluOp::Or, true) => ebpf::OR64_REG,
        (AluOp::And, false) => ebpf::AND64_IMM,
        (AluOp::And, true) => ebpf::AND64_REG,
        (AluOp::Xor, false) => ebpf::XOR64_IMM,
        (AluOp::Xor, true) => ebpf::XOR64_REG,
        (AluOp::Lsh, false) => ebpf::LSH64_IMM,
        (AluOp::Lsh, true) => ebpf::LSH64_REG,
        (AluOp::Rsh, false) => ebpf::RSH64_IMM,
        (AluOp::Rsh, true) => ebpf::RSH64_REG,
        (AluOp::Arsh, false) => ebpf::ARSH64_IMM,
        (AluOp::Arsh, true) => ebpf::ARSH64_REG,
    };
    match src {
        Value::Imm(imm) => build_insn(opc, dst, Reg::R0, 0, *imm),
        Value::Register(reg) => build_insn(opc, dst, *reg, 0, 0),
    }
}

fn lower_load(dst: Reg, base: Reg, offset: i16, size: MemSize) -> ebpf::Insn {
    let opc = match size {
        MemSize::B1 => ebpf::LD_B_REG,
        MemSize::B2 => ebpf::LD_H_REG,
        MemSize::B4 => ebpf::LD_W_REG,
        MemSize::B8 => ebpf::LD_DW_REG,
    };
    build_insn(opc, dst, base, offset, 0)
}

fn lower_store(base: Reg, offset: i16, src: &Value, size: MemSize) -> ebpf::Insn {
    let reg = value_is_register(src);
    let opc = match (size, reg) {
        (MemSize::B1, false) => ebpf::ST_B_IMM,
        (MemSize::B2, false) => ebpf::ST_H_IMM,
        (MemSize::B4, false) => ebpf::ST_W_IMM,
        (MemSize::B8, false) => ebpf::ST_DW_IMM,
        (MemSize::B1, true) => ebpf::ST_B_REG,
        (MemSize::B2, true) => ebpf::ST_H_REG,
        (MemSize::B4, true) => ebpf::ST_W_REG,
        (MemSize::B8, true) => ebpf::ST_DW_REG,
    };
    match src {
        Value::Imm(imm) => build_insn(opc, base, Reg::R0, offset, *imm),
        Value::Register(reg) => build_insn(opc, base, *reg, offset, 0),
    }
}

fn lower_cond(cond: Cond, rhs: &Value) -> u8 {
    let reg = value_is_register(rhs);
    match (cond, reg) {
        (Cond::Eq, false) => ebpf::JEQ64_IMM,
        (Cond::Eq, true) => ebpf::JEQ64_REG,
        (Cond::Ne, false) => ebpf::JNE64_IMM,
        (Cond::Ne, true) => ebpf::JNE64_REG,
        (Cond::Gt, false) => ebpf::JSGT64_IMM,
        (Cond::Gt, true) => ebpf::JSGT64_REG,
        (Cond::Ge, false) => ebpf::JSGE64_IMM,
        (Cond::Ge, true) => ebpf::JSGE64_REG,
        (Cond::Lt, false) => ebpf::JSLT64_IMM,
        (Cond::Lt, true) => ebpf::JSLT64_REG,
        (Cond::Le, false) => ebpf::JSLE64_IMM,
        (Cond::Le, true) => ebpf::JSLE64_REG,
        (Cond::Gtu, false) => ebpf::JGT64_IMM,
        (Cond::Gtu, true) => ebpf::JGT64_REG,
        (Cond::Geu, false) => ebpf::JGE64_IMM,
        (Cond::Geu, true) => ebpf::JGE64_REG,
        (Cond::Ltu, false) => ebpf::JLT64_IMM,
        (Cond::Ltu, true) => ebpf::JLT64_REG,
        (Cond::Leu, false) => ebpf::JLE64_IMM,
        (Cond::Leu, true) => ebpf::JLE64_REG,
    }
}

fn syscall_hash(id: SyscallId) -> u32 {
    let name = match id {
        SyscallId::SolLog => b"sol_log".as_slice(),
        SyscallId::SolLogData => b"sol_log_data".as_slice(),
        SyscallId::SolLogPubkey => b"sol_log_pubkey".as_slice(),
        SyscallId::SolAllocFree => b"sol_alloc_free_".as_slice(),
        SyscallId::SolMemcpy => b"sol_memcpy_".as_slice(),
        SyscallId::SolMemset => b"sol_memset_".as_slice(),
        SyscallId::SolMemmove => b"sol_memmove_".as_slice(),
        SyscallId::SolMemcmp => b"sol_memcmp_".as_slice(),
        SyscallId::Abort => b"abort".as_slice(),
        SyscallId::Panic => b"sol_panic_".as_slice(),
    };
    ebpf::hash_symbol_name(name)
}

fn approx_stack_depth(base: Reg, offset: i16, size: MemSize) -> Option<u32> {
    if base != Reg::FP || offset >= 0 {
        return None;
    }
    let bytes: u32 = match size {
        MemSize::B1 => 1,
        MemSize::B2 => 2,
        MemSize::B4 => 4,
        MemSize::B8 => 8,
    };
    let off = u32::try_from(-i32::from(offset)).ok()?;
    Some(off.saturating_add(bytes.saturating_sub(1)))
}

fn terminal(instr: &IrInstr) -> bool {
    matches!(instr, IrInstr::Return)
}

fn estimate_len(instr: &IrInstr) -> usize {
    match instr {
        IrInstr::FakeDep { strategy, .. } => match strategy {
            crate::ir::FakeDepStrategy::XorZero => 1,
            crate::ir::FakeDepStrategy::MovSelf => 1,
            crate::ir::FakeDepStrategy::AddSubPair => 2,
        },
        IrInstr::StackPressure { bytes, strategy } => match strategy {
            StackPressureStrategy::DeadAlloc => ((*bytes as usize) / 8).max(1),
            StackPressureStrategy::SpillReload { .. } => 2,
            StackPressureStrategy::DeepNesting { depth } => *depth as usize,
        },
        IrInstr::AliasProbe { alias_class, .. } => match alias_class {
            AliasClass::CrossRegion => 4,
            AliasClass::StackOverlap { .. } | AliasClass::InputRegion { .. } => 4,
        },
        _ => 1,
    }
}

fn rel_offset(current_insn: usize, target_insn: usize) -> Result<i16, LoweringError> {
    let delta = target_insn as i64 - current_insn as i64 - 1;
    i16::try_from(delta).map_err(|_| LoweringError::OffsetOutOfRange)
}

/// Lower IR into sBPF bytecode.
pub fn lower(ir: &IR, config: &LoweringConfig) -> Result<Vec<u8>, LoweringError> {
    if let Err(errors) = validate(ir) {
        return Err(LoweringError::ValidationFailed { errors });
    }

    let mut regions = ir.regions.clone();
    if let Some(last_region) = regions.last_mut() {
        let needs_exit = last_region
            .instructions
            .last()
            .map(|instr| !terminal(instr))
            .unwrap_or(true);
        if needs_exit {
            last_region.instructions.push(IrInstr::Return);
        }
    }

    let mut region_starts = Vec::with_capacity(regions.len());
    let mut cursor = 0usize;
    for region in &regions {
        region_starts.push(cursor);
        for instr in &region.instructions {
            cursor = cursor.saturating_add(estimate_len(instr));
        }
    }

    let mut insns = Vec::<ebpf::Insn>::new();
    let mut max_depth_seen = 0u32;

    for region in &regions {
        for instr in &region.instructions {
            match instr {
                IrInstr::Alu { dst, op, src } => emit(&mut insns, lower_alu(*dst, *op, src)),
                IrInstr::Mov { dst, src } => emit(&mut insns, lower_mov(*dst, src)),
                IrInstr::Load {
                    dst,
                    base,
                    offset,
                    size,
                } => emit(&mut insns, lower_load(*dst, *base, *offset, *size)),
                IrInstr::Store {
                    base,
                    offset,
                    src,
                    size,
                } => {
                    if let Some(depth) = approx_stack_depth(*base, *offset, *size) {
                        max_depth_seen = max_depth_seen.max(depth);
                    }
                    emit(&mut insns, lower_store(*base, *offset, src, *size));
                }
                IrInstr::Br {
                    cond,
                    lhs,
                    rhs,
                    target,
                } => {
                    let target_insn = region_starts[*target];
                    let current = insns.len();
                    let off = rel_offset(current, target_insn)?;
                    let opc = lower_cond(*cond, rhs);
                    let insn = match rhs {
                        Value::Imm(imm) => build_insn(opc, *lhs, Reg::R0, off, *imm),
                        Value::Register(reg) => build_insn(opc, *lhs, *reg, off, 0),
                    };
                    emit(&mut insns, insn);
                }
                IrInstr::BrUncond { target } => {
                    let target_insn = region_starts[*target];
                    let current = insns.len();
                    let off = rel_offset(current, target_insn)?;
                    emit(&mut insns, build_insn(ebpf::JA, Reg::R0, Reg::R0, off, 0));
                }
                IrInstr::Call { target } => {
                    let target_insn = region_starts[*target];
                    let current = insns.len();
                    let rel = target_insn as i64 - current as i64 - 1;
                    emit(
                        &mut insns,
                        build_insn(
                            ebpf::CALL_IMM,
                            Reg::R0,
                            Reg::R1,
                            0,
                            rel,
                        ),
                    );
                }
                IrInstr::Return => emit(&mut insns, build_insn(ebpf::EXIT, Reg::R0, Reg::R0, 0, 0)),
                IrInstr::Syscall { id, args } => {
                    let arg_regs = [Reg::R1, Reg::R2, Reg::R3, Reg::R4, Reg::R5];
                    for (idx, arg) in args.iter().enumerate() {
                        if let Some(value) = arg {
                            emit(&mut insns, lower_mov(arg_regs[idx], value));
                        }
                    }
                    emit(
                        &mut insns,
                        build_insn(
                            ebpf::CALL_IMM,
                            Reg::R0,
                            Reg::R0,
                            0,
                            i64::from(syscall_hash(*id)),
                        ),
                    );
                }
                IrInstr::FakeDep { reg, strategy } => match strategy {
                    crate::ir::FakeDepStrategy::XorZero => {
                        emit(&mut insns, build_insn(ebpf::XOR64_IMM, *reg, Reg::R0, 0, 0));
                    }
                    crate::ir::FakeDepStrategy::MovSelf => {
                        emit(&mut insns, build_insn(ebpf::MOV64_REG, *reg, *reg, 0, 0));
                    }
                    crate::ir::FakeDepStrategy::AddSubPair => {
                        emit(&mut insns, build_insn(ebpf::ADD64_IMM, *reg, Reg::R0, 0, 1));
                        emit(&mut insns, build_insn(ebpf::SUB64_IMM, *reg, Reg::R0, 0, 1));
                    }
                },
                IrInstr::StackPressure { bytes, strategy } => {
                    let claimed_depth = *bytes;
                    max_depth_seen = max_depth_seen.max(claimed_depth);
                    match strategy {
                        StackPressureStrategy::DeadAlloc => {
                            let mut slots = ((*bytes as usize) / 8).max(1);
                            let mut off = -8i16;
                            while slots > 0 {
                                emit(&mut insns, build_insn(ebpf::ST_DW_REG, Reg::FP, Reg::R0, off, 0));
                                off = off.saturating_sub(8);
                                slots -= 1;
                            }
                        }
                        StackPressureStrategy::SpillReload { reg } => {
                            emit(&mut insns, build_insn(ebpf::ST_DW_REG, Reg::FP, *reg, -8, 0));
                            emit(&mut insns, build_insn(ebpf::LD_DW_REG, *reg, Reg::FP, -8, 0));
                        }
                        StackPressureStrategy::DeepNesting { depth } => {
                            for _ in 0..*depth {
                                emit(&mut insns, build_insn(ebpf::CALL_IMM, Reg::R0, Reg::R1, 0, 0));
                            }
                        }
                    }
                }
                IrInstr::AliasProbe { ptr, alias_class } => {
                    let second = if *ptr == Reg::R7 { Reg::R8 } else { Reg::R7 };
                    match alias_class {
                        AliasClass::StackOverlap { offset_a, offset_b } => {
                            emit(&mut insns, build_insn(ebpf::MOV64_REG, *ptr, Reg::FP, 0, 0));
                            emit(&mut insns, build_insn(ebpf::ADD64_IMM, *ptr, Reg::R0, 0, i64::from(*offset_a)));
                            emit(&mut insns, build_insn(ebpf::MOV64_REG, second, Reg::FP, 0, 0));
                            emit(&mut insns, build_insn(ebpf::ADD64_IMM, second, Reg::R0, 0, i64::from(*offset_b)));
                        }
                        AliasClass::InputRegion { offset_a, offset_b } => {
                            emit(&mut insns, build_insn(ebpf::MOV64_REG, *ptr, Reg::R1, 0, 0));
                            emit(&mut insns, build_insn(ebpf::ADD64_IMM, *ptr, Reg::R0, 0, i64::from(*offset_a)));
                            emit(&mut insns, build_insn(ebpf::MOV64_REG, second, Reg::R1, 0, 0));
                            emit(&mut insns, build_insn(ebpf::ADD64_IMM, second, Reg::R0, 0, i64::from(*offset_b)));
                        }
                        AliasClass::CrossRegion => {
                            emit(&mut insns, build_insn(ebpf::MOV64_REG, *ptr, Reg::FP, 0, 0));
                            emit(&mut insns, build_insn(ebpf::ADD64_IMM, *ptr, Reg::R0, 0, -16));
                            emit(&mut insns, build_insn(ebpf::MOV64_REG, second, Reg::R1, 0, 0));
                            emit(&mut insns, build_insn(ebpf::ADD64_IMM, second, Reg::R0, 0, 16));
                        }
                    }
                }
            }
            if max_depth_seen > config.max_stack_depth {
                return Err(LoweringError::StackOverflow {
                    depth: max_depth_seen,
                    max: config.max_stack_depth,
                });
            }
        }
    }

    if insns.len() > config.max_insn_count {
        return Err(LoweringError::InsnLimitExceeded {
            count: insns.len(),
            max: config.max_insn_count,
        });
    }

    let mut bytes = Vec::with_capacity(insns.len() * ebpf::INSN_SIZE);
    for insn in insns {
        bytes.extend_from_slice(&insn.to_array());
    }
    Ok(bytes)
}
