//! IR validation.

use crate::ir::{IR, IrInstr, Reg};

/// Validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A destination attempted to write to frame pointer.
    WriteToFramePointer {
        /// Region index containing the error.
        region: usize,
        /// Instruction index within region.
        instruction: usize,
    },
    /// A control-flow target points outside available regions.
    BranchTargetOutOfBounds {
        /// Region index containing the error.
        region: usize,
        /// Instruction index within region.
        instruction: usize,
        /// Invalid target index.
        target: usize,
        /// Region count.
        region_count: usize,
    },
    /// Entry region index is not valid.
    EntryRegionOutOfBounds {
        /// Invalid entry index.
        entry_region: usize,
        /// Region count.
        region_count: usize,
    },
    /// Syscall args have a positional gap.
    SyscallArgGap {
        /// Region index containing the error.
        region: usize,
        /// Instruction index within region.
        instruction: usize,
        /// First argument index with a gap.
        arg_index: usize,
    },
    /// Region contains no instructions.
    EmptyRegion {
        /// Region index.
        region: usize,
    },
}

fn is_control_terminal(instr: &IrInstr) -> bool {
    matches!(instr, IrInstr::Br { .. } | IrInstr::BrUncond { .. } | IrInstr::Return | IrInstr::Call { .. })
}

/// Validate structural properties of an IR program.
pub fn validate(ir: &IR) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if ir.entry_region >= ir.regions.len() {
        errors.push(ValidationError::EntryRegionOutOfBounds {
            entry_region: ir.entry_region,
            region_count: ir.regions.len(),
        });
    }

    let region_count = ir.regions.len();
    for (region_idx, region) in ir.regions.iter().enumerate() {
        if region.instructions.is_empty() {
            errors.push(ValidationError::EmptyRegion { region: region_idx });
            continue;
        }

        for (insn_idx, instr) in region.instructions.iter().enumerate() {
            match instr {
                IrInstr::Alu { dst, .. } | IrInstr::Mov { dst, .. } | IrInstr::Load { dst, .. }
                    if *dst == Reg::FP =>
                {
                    errors.push(ValidationError::WriteToFramePointer {
                        region: region_idx,
                        instruction: insn_idx,
                    });
                }
                IrInstr::FakeDep { reg, .. } if *reg == Reg::FP => {
                    errors.push(ValidationError::WriteToFramePointer {
                        region: region_idx,
                        instruction: insn_idx,
                    });
                }
                IrInstr::Br { target, .. } | IrInstr::BrUncond { target } | IrInstr::Call { target }
                    if *target >= region_count =>
                {
                    errors.push(ValidationError::BranchTargetOutOfBounds {
                        region: region_idx,
                        instruction: insn_idx,
                        target: *target,
                        region_count,
                    });
                }
                IrInstr::Syscall { args, .. } => {
                    let mut saw_none = false;
                    for (arg_index, arg) in args.iter().enumerate() {
                        if arg.is_none() {
                            saw_none = true;
                            continue;
                        }
                        if saw_none {
                            errors.push(ValidationError::SyscallArgGap {
                                region: region_idx,
                                instruction: insn_idx,
                                arg_index,
                            });
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        if region_idx + 1 == region_count {
            continue;
        }
        if let Some(last) = region.instructions.last() {
            let _warn_only = !is_control_terminal(last);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
