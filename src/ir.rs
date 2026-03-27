//! Core IR definitions for sBPF fuzzing.

/// sBPF register identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reg {
    /// Return value register.
    R0,
    /// Argument register.
    R1,
    /// Argument register.
    R2,
    /// Argument register.
    R3,
    /// Argument register.
    R4,
    /// Argument register.
    R5,
    /// Callee-save/general register.
    R6,
    /// Callee-save/general register.
    R7,
    /// Callee-save/general register.
    R8,
    /// Callee-save/general register.
    R9,
    /// Frame pointer register.
    FP,
}

impl Reg {
    /// Returns raw register index.
    pub fn index(&self) -> u8 {
        match self {
            Reg::R0 => 0,
            Reg::R1 => 1,
            Reg::R2 => 2,
            Reg::R3 => 3,
            Reg::R4 => 4,
            Reg::R5 => 5,
            Reg::R6 => 6,
            Reg::R7 => 7,
            Reg::R8 => 8,
            Reg::R9 => 9,
            Reg::FP => 10,
        }
    }

    /// Returns whether register can be written.
    pub fn is_writable(&self) -> bool {
        !matches!(self, Reg::FP)
    }
}

/// Instruction source value.
#[derive(Debug, Clone)]
pub enum Value {
    /// Immediate value.
    Imm(i64),
    /// Register source.
    Register(Reg),
}

/// ALU operations.
#[derive(Debug, Clone, Copy)]
pub enum AluOp {
    /// Add.
    Add,
    /// Subtract.
    Sub,
    /// Multiply.
    Mul,
    /// Divide.
    Div,
    /// Modulo.
    Mod,
    /// Bitwise OR.
    Or,
    /// Bitwise AND.
    And,
    /// Bitwise XOR.
    Xor,
    /// Left shift.
    Lsh,
    /// Logical right shift.
    Rsh,
    /// Arithmetic right shift.
    Arsh,
}

/// Branch condition.
#[derive(Debug, Clone, Copy)]
pub enum Cond {
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
    /// Signed greater than.
    Gt,
    /// Signed greater or equal.
    Ge,
    /// Signed less than.
    Lt,
    /// Signed less or equal.
    Le,
    /// Unsigned greater than.
    Gtu,
    /// Unsigned greater or equal.
    Geu,
    /// Unsigned less than.
    Ltu,
    /// Unsigned less or equal.
    Leu,
}

/// Memory width.
#[derive(Debug, Clone, Copy)]
pub enum MemSize {
    /// 1-byte access.
    B1,
    /// 2-byte access.
    B2,
    /// 4-byte access.
    B4,
    /// 8-byte access.
    B8,
}

/// Selected syscall IDs.
#[derive(Debug, Clone, Copy)]
pub enum SyscallId {
    /// sol_log
    SolLog,
    /// sol_log_data
    SolLogData,
    /// sol_log_pubkey
    SolLogPubkey,
    /// sol_alloc_free
    SolAllocFree,
    /// sol_memcpy
    SolMemcpy,
    /// sol_memset
    SolMemset,
    /// sol_memmove
    SolMemmove,
    /// sol_memcmp
    SolMemcmp,
    /// abort
    Abort,
    /// panic
    Panic,
}

/// Fake dependency expansion strategy.
#[derive(Debug, Clone, Copy)]
pub enum FakeDepStrategy {
    /// Emit xor with zero.
    XorZero,
    /// Emit self move.
    MovSelf,
    /// Emit add/sub identity pair.
    AddSubPair,
}

/// Stack pressure expansion strategy.
#[derive(Debug, Clone)]
pub enum StackPressureStrategy {
    /// Dead stack writes.
    DeadAlloc,
    /// Spill then reload selected register.
    SpillReload {
        /// Register to spill and reload.
        reg: Reg,
    },
    /// Deep internal call nesting.
    DeepNesting {
        /// Call nesting depth.
        depth: u32,
    },
}

/// Alias categories for pointer probes.
#[derive(Debug, Clone)]
pub enum AliasClass {
    /// Overlapping stack pointers.
    StackOverlap {
        /// First stack offset.
        offset_a: i16,
        /// Second stack offset.
        offset_b: i16,
    },
    /// Two input-region pointers.
    InputRegion {
        /// First input offset.
        offset_a: i16,
        /// Second input offset.
        offset_b: i16,
    },
    /// One stack and one input pointer.
    CrossRegion,
}

/// IR instruction.
#[derive(Debug, Clone)]
pub enum IrInstr {
    /// ALU operation.
    Alu {
        /// Destination register.
        dst: Reg,
        /// Arithmetic/logical operation.
        op: AluOp,
        /// Source operand.
        src: Value,
    },
    /// Move operation.
    Mov {
        /// Destination register.
        dst: Reg,
        /// Source operand.
        src: Value,
    },
    /// Memory load.
    Load {
        /// Destination register.
        dst: Reg,
        /// Base register.
        base: Reg,
        /// Offset from base.
        offset: i16,
        /// Access width.
        size: MemSize,
    },
    /// Memory store.
    Store {
        /// Base register.
        base: Reg,
        /// Offset from base.
        offset: i16,
        /// Source value.
        src: Value,
        /// Access width.
        size: MemSize,
    },
    /// Conditional branch.
    Br {
        /// Branch condition.
        cond: Cond,
        /// LHS register.
        lhs: Reg,
        /// RHS value.
        rhs: Value,
        /// Target region index.
        target: usize,
    },
    /// Unconditional branch.
    BrUncond {
        /// Target region index.
        target: usize,
    },
    /// Internal call.
    Call {
        /// Target region index.
        target: usize,
    },
    /// Function return.
    Return,
    /// Syscall invocation.
    Syscall {
        /// Syscall kind.
        id: SyscallId,
        /// Positional args for r1-r5.
        args: [Option<Value>; 5],
    },
    /// Fake dependency meta instruction.
    FakeDep {
        /// Register to create dependency on.
        reg: Reg,
        /// Expansion strategy.
        strategy: FakeDepStrategy,
    },
    /// Stack pressure meta instruction.
    StackPressure {
        /// Requested bytes.
        bytes: u32,
        /// Expansion strategy.
        strategy: StackPressureStrategy,
    },
    /// Alias analysis meta instruction.
    AliasProbe {
        /// Primary pointer register to set up.
        ptr: Reg,
        /// Alias setup class.
        alias_class: AliasClass,
    },
}

/// Basic region in a sequence.
#[derive(Debug, Clone)]
pub struct BasicRegion {
    /// Human readable label.
    pub label: String,
    /// Region instructions.
    pub instructions: Vec<IrInstr>,
}

/// Full IR sequence.
#[derive(Debug, Clone)]
pub struct Sequence {
    /// Regions in layout order.
    pub regions: Vec<BasicRegion>,
    /// Entry region index.
    pub entry_region: usize,
}

/// Public IR alias.
pub type IR = Sequence;
