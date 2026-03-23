use crate::ir::{sbpf2ir, IrNode, IrSeq};
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use std::path::Path;

fn save_ir(path: &str, ir: &IrSeq) {
    let bytes = bincode::serialize(ir).expect("failed to serialize IR");
    std::fs::write(path, bytes).expect("failed to write IR file");
}

fn ir_push(ir: &mut IrSeq, op: &str, operands: &[&str]) {
    let last = ir.regions.len() - 1;
    ir.regions[last].1.push(IrNode {
        op: op.to_string(),
        operands: operands.iter().map(|s| s.to_string()).collect(),
    });
}

fn version_str(idx: usize) -> &'static str {
    match idx % 5 {
        0 => "V0",
        1 => "V1",
        2 => "V2",
        3 => "V3",
        _ => "V4",
    }
}

/// Returns true if the version uses PQR instructions (V2 only) instead of mul/div/mod.
fn uses_pqr(version: &str) -> bool {
    version == "V2"
}

fn reg(n: u8) -> String {
    format!("r{}", n)
}

fn imm(v: i64) -> String {
    if v < 0 {
        format!("{}", v)
    } else {
        format!("0x{:x}", v)
    }
}

// --- Template 1: Register Pressure ---
// All r0-r9 active, 30-60 reg-reg ALU ops in one region.
fn gen_register_pressure(version: &str, rng: &mut impl Rng) -> IrSeq {
    let mut ir = IrSeq::new(version, vec![]);
    let count = rng.gen_range(30..=60);

    // Initialize all registers
    for r in 0..10u8 {
        let v = rng.gen_range(1..=0xffi64);
        ir_push(&mut ir, "mov64", &[&reg(r), &imm(v)]);
    }

    let alu_ops: &[&str] = if uses_pqr(version) {
        &[
            "add64", "sub64", "or64", "and64", "lsh64", "rsh64", "xor64", "arsh64",
        ]
    } else {
        &[
            "add64", "sub64", "mul64", "or64", "and64", "lsh64", "rsh64", "xor64", "arsh64",
        ]
    };

    for _ in 0..count {
        let op = alu_ops[rng.gen_range(0..alu_ops.len())];
        let dst = rng.gen_range(0..10u8);
        let src = rng.gen_range(0..10u8);
        // For shifts, use immediate to keep in range
        if op.contains("sh") {
            let shift = rng.gen_range(0..64i64);
            ir_push(&mut ir, op, &[&reg(dst), &imm(shift)]);
        } else {
            ir_push(&mut ir, op, &[&reg(dst), &reg(src)]);
        }
    }

    ir_push(&mut ir, "exit", &[]);
    ir
}

// --- Template 2: Metering Stress ---
// 50-200 straight-line expensive ops (no branches/calls).
fn gen_metering_stress(version: &str, rng: &mut impl Rng) -> IrSeq {
    let mut ir = IrSeq::new(version, vec![]);
    let count = rng.gen_range(50..=200);

    // Setup
    ir_push(&mut ir, "mov64", &["r1", "0x1"]);
    ir_push(&mut ir, "mov64", &["r2", "0x2"]);
    ir_push(&mut ir, "mov64", &["r3", "0x3"]);
    ir_push(&mut ir, "mov64", &["r4", "0x7"]);
    ir_push(&mut ir, "mov64", &["r5", "0xff"]);
    ir_push(&mut ir, "mov64", &["r0", "0x0"]);

    let ops: &[&str] = if uses_pqr(version) {
        &[
            "add64", "sub64", "or64", "and64", "xor64", "lmul64", "udiv64", "urem64",
        ]
    } else {
        &[
            "add64", "sub64", "mul64", "div64", "mod64", "or64", "and64", "xor64",
        ]
    };

    for _ in 0..count {
        let op = ops[rng.gen_range(0..ops.len())];
        let dst = rng.gen_range(0..6u8);
        // For div/udiv/mod/urem, use immediate to avoid div-by-zero
        if op.contains("div") || op.contains("mod") || op.contains("rem") {
            let divisor = rng.gen_range(1..=255i64);
            ir_push(&mut ir, op, &[&reg(dst), &imm(divisor)]);
        } else {
            let src = rng.gen_range(0..6u8);
            ir_push(&mut ir, op, &[&reg(dst), &reg(src)]);
        }
    }

    ir_push(&mut ir, "exit", &[]);
    ir
}

// --- Template 3: Jump Back-Patching ---
// 10-20 function regions with inter-region calls.
// Uses assembly text + sbpf2ir() for proper label resolution.
fn gen_jump_backpatch(version: &str, rng: &mut impl Rng) -> Option<IrSeq> {
    let num_funcs = rng.gen_range(3..=8);
    let mut asm = String::new();

    // Entrypoint: call each function sequentially
    for i in 0..num_funcs {
        asm.push_str(&format!("call function_bp_{}\n", i));
    }
    asm.push_str("mov64 r0, 0x0\nexit\n");

    // Each function region: some ALU + return via exit
    let ops: &[&str] = if uses_pqr(version) {
        &["add64", "sub64", "or64", "and64", "xor64"]
    } else {
        &["add64", "sub64", "mul64", "or64", "and64"]
    };

    for i in 0..num_funcs {
        asm.push_str(&format!("function_bp_{}:\n", i));
        let insn_count = rng.gen_range(3..=10);
        for _ in 0..insn_count {
            let op = ops[rng.gen_range(0..ops.len())];
            let dst = rng.gen_range(0..10u8);
            let v = rng.gen_range(1..=100i64);
            asm.push_str(&format!("{} r{}, {}\n", op, dst, v));
        }
        asm.push_str("exit\n");
    }

    Some(sbpf2ir(&asm, vec![], version))
}

// --- Template 4: Deep Call Chains ---
// Depth 5-10 nested function calls with callee-saved register verification.
fn gen_deep_calls(version: &str, rng: &mut impl Rng) -> Option<IrSeq> {
    let depth = rng.gen_range(3..=6);
    let mut asm = String::new();

    // Entrypoint: set up r6-r9 with known values, call depth_0, verify on return
    asm.push_str("mov64 r6, 0x42\n");
    asm.push_str("mov64 r7, 0x43\n");
    asm.push_str("mov64 r8, 0x44\n");
    asm.push_str("mov64 r9, 0x45\n");
    asm.push_str("call function_depth_0\n");
    // After return, r6-r9 should be preserved
    asm.push_str("mov64 r0, r6\n");
    asm.push_str("exit\n");

    let ops: &[&str] = if uses_pqr(version) {
        &["add64", "sub64", "or64", "xor64"]
    } else {
        &["add64", "sub64", "mul64", "or64"]
    };

    for i in 0..depth {
        asm.push_str(&format!("function_depth_{}:\n", i));
        // Each function does some work with callee-saved regs
        let work = rng.gen_range(2..=5);
        for _ in 0..work {
            let op = ops[rng.gen_range(0..ops.len())];
            let dst = rng.gen_range(6..=9u8);
            let v = rng.gen_range(1..=50i64);
            asm.push_str(&format!("{} r{}, {}\n", op, dst, v));
        }
        if i + 1 < depth {
            asm.push_str(&format!("call function_depth_{}\n", i + 1));
        }
        asm.push_str("exit\n");
    }

    Some(sbpf2ir(&asm, vec![], version))
}

// --- Template 5: Memory Patterns ---
// 20-50 mixed load/store at varying offsets and sizes, with input memory.
fn gen_memory_patterns(version: &str, rng: &mut impl Rng) -> IrSeq {
    let count = rng.gen_range(20..=50);
    // Provide some input memory bytes for loads to read
    let mem_size: usize = rng.gen_range(32..=128);
    let memory: Vec<u8> = (0..mem_size).map(|_| rng.gen()).collect();
    let mut ir = IrSeq::new(version, memory);

    // r1 = MM_INPUT_START (set by VM), use it as base pointer
    // Do stores and loads at various offsets within our memory region
    let store_ops = &["stb", "sth", "stw", "stdw"];
    let load_ops = &["ldxb", "ldxh", "ldxw", "ldxdw"];
    let sizes: &[usize] = &[1, 2, 4, 8];

    for _ in 0..count {
        let is_store = rng.gen_bool(0.4);
        if is_store {
            let size_idx = rng.gen_range(0..4);
            let op = store_ops[size_idx];
            let alignment = sizes[size_idx];
            let max_off = mem_size.saturating_sub(alignment);
            if max_off == 0 {
                continue;
            }
            let off = (rng.gen_range(0..max_off) / alignment) * alignment;
            let val = rng.gen_range(0..=0xffi64);
            ir_push(
                &mut ir,
                op,
                &[&format!("[r1+0x{:x}]", off), &imm(val)],
            );
        } else {
            let size_idx = rng.gen_range(0..4);
            let op = load_ops[size_idx];
            let alignment = sizes[size_idx];
            let max_off = mem_size.saturating_sub(alignment);
            if max_off == 0 {
                continue;
            }
            let off = (rng.gen_range(0..max_off) / alignment) * alignment;
            let dst = rng.gen_range(0..10u8);
            ir_push(
                &mut ir,
                op,
                &[&reg(dst), &format!("[r1+0x{:x}]", off)],
            );
        }
    }

    ir_push(&mut ir, "mov64", &["r0", "0x0"]);
    ir_push(&mut ir, "exit", &[]);
    ir
}

// --- Template 6: Arithmetic Edge Cases ---
// Edge-value registers + boundary shifts + PQR for V2 / old div for V0.
fn gen_arithmetic_edge(version: &str, rng: &mut impl Rng) -> IrSeq {
    let mut ir = IrSeq::new(version, vec![]);

    // Load edge values into registers
    // All values must fit in i32 range for the assembler
    let edge_vals: &[i64] = &[0, 1, -1, 0x7fffffff, -2147483648, 0x7fff, 0xff, 0xffff];
    for r in 0..8u8 {
        let v = edge_vals[r as usize];
        ir_push(&mut ir, "mov64", &[&reg(r), &imm(v)]);
    }
    // r8, r9 for scratch
    ir_push(&mut ir, "mov64", &["r8", "0x1"]);
    ir_push(&mut ir, "mov64", &["r9", "0x2"]);

    // Boundary shifts: 0, 1, 31, 32, 63
    let shift_amounts = &[0i64, 1, 31, 32, 63];
    for &shift in shift_amounts {
        let dst = rng.gen_range(0..8u8);
        ir_push(&mut ir, "lsh64", &[&reg(dst), &imm(shift)]);
        let dst = rng.gen_range(0..8u8);
        ir_push(&mut ir, "rsh64", &[&reg(dst), &imm(shift)]);
        let dst = rng.gen_range(0..8u8);
        ir_push(&mut ir, "arsh64", &[&reg(dst), &imm(shift)]);
    }

    // 32-bit shifts at boundaries: 0, 1, 31
    for &shift in &[0i64, 1, 31] {
        let dst = rng.gen_range(0..8u8);
        ir_push(&mut ir, "lsh32", &[&reg(dst), &imm(shift)]);
        let dst = rng.gen_range(0..8u8);
        ir_push(&mut ir, "rsh32", &[&reg(dst), &imm(shift)]);
        let dst = rng.gen_range(0..8u8);
        ir_push(&mut ir, "arsh32", &[&reg(dst), &imm(shift)]);
    }

    // Version-specific arithmetic
    if uses_pqr(version) {
        let pqr_ops = &["lmul64", "udiv64", "urem64", "sdiv64", "srem64"];
        for op in pqr_ops {
            let dst = rng.gen_range(0..8u8);
            // Use immediate to avoid div-by-zero
            let v = rng.gen_range(1..=255i64);
            ir_push(&mut ir, op, &[&reg(dst), &imm(v)]);
        }
        // 32-bit PQR
        let pqr32_ops = &["lmul32", "udiv32", "urem32", "sdiv32", "srem32"];
        for op in pqr32_ops {
            let dst = rng.gen_range(0..8u8);
            let v = rng.gen_range(1..=255i64);
            ir_push(&mut ir, op, &[&reg(dst), &imm(v)]);
        }
    } else {
        // Old-style mul/div/mod
        let old_ops = &["mul64", "div64", "mod64"];
        for op in old_ops {
            let dst = rng.gen_range(0..8u8);
            let v = rng.gen_range(1..=255i64);
            ir_push(&mut ir, op, &[&reg(dst), &imm(v)]);
        }
        let old32 = &["mul32", "div32", "mod32"];
        for op in old32 {
            let dst = rng.gen_range(0..8u8);
            let v = rng.gen_range(1..=255i64);
            ir_push(&mut ir, op, &[&reg(dst), &imm(v)]);
        }
    }

    ir_push(&mut ir, "exit", &[]);
    ir
}

/// Generate JIT-stress IR seeds.
/// Removes old `jit_stress_*.ir` files, generates `count` seeds across all 5 SBPF versions
/// round-robining through 6 template categories.
/// Returns the number of seeds generated.
pub fn generate(output_dir: &str, count: usize, seed: u64) -> usize {
    let path = Path::new(output_dir);
    std::fs::create_dir_all(path).unwrap();

    // Remove old jit_stress seeds
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name
                .to_str()
                .map_or(false, |n| n.starts_with("jit_stress_") && n.ends_with(".ir"))
            {
                std::fs::remove_file(entry.path()).ok();
            }
        }
    }

    let mut rng = StdRng::seed_from_u64(seed);
    let mut generated = 0;

    let template_names = [
        "regpress",
        "metering",
        "backpatch",
        "deepcall",
        "mempat",
        "arithedge",
    ];

    for i in 0..count {
        let version = version_str(i);
        let template_idx = i % template_names.len();

        let ir = match template_idx {
            0 => Some(gen_register_pressure(version, &mut rng)),
            1 => Some(gen_metering_stress(version, &mut rng)),
            2 => gen_jump_backpatch(version, &mut rng),
            3 => gen_deep_calls(version, &mut rng),
            4 => Some(gen_memory_patterns(version, &mut rng)),
            _ => Some(gen_arithmetic_edge(version, &mut rng)),
        };

        if let Some(ir) = ir {
            let name = format!(
                "jit_stress_{}_{:04}_{}.ir",
                template_names[template_idx],
                generated,
                version.to_lowercase()
            );
            save_ir(path.join(&name).to_str().unwrap(), &ir);
            generated += 1;
        }
    }

    println!(
        "Generated {} JIT-stress IR seeds in {}",
        generated, output_dir
    );
    generated
}
