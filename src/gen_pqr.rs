use crate::ir::{sbpf2ir, IrSeq};
use std::path::Path;

fn save_ir(path: &str, ir: &IrSeq) {
    let bytes = bincode::serialize(ir).expect("failed to serialize IR");
    std::fs::write(path, bytes).expect("failed to write IR file");
}

/// Mirrors test_pqr_v0 (execution.rs line 486).
/// V0 uses lddw for 64-bit loads.
fn gen_v0_pqr(output_dir: &Path, idx: &mut usize) {
    // (mnemonic, dst, src) tuples from the test
    let cases: Vec<(&str, u64, u64)> = vec![
        ("div32", 13, 4),
        ("div64", 13, 4),
        ("mod32", 13, 4),
        ("mod64", 13, 4),
        ("div32", 13, u32::MAX as u64),
        ("div64", 13, u32::MAX as u64),
        ("mod32", 13, u32::MAX as u64),
        ("mod64", 13, u32::MAX as u64),
        ("div32", u64::MAX, 4),
        ("div64", u64::MAX, 4),
        ("mod32", u64::MAX, 4),
        ("mod64", u64::MAX, 4),
        ("div32", u64::MAX, u32::MAX as u64),
        // Skipping DIV64 u64::MAX / u32::MAX (sign extension mismatch in V0, commented out in test)
        ("mod32", u64::MAX, u32::MAX as u64),
        ("mod64", u64::MAX, u32::MAX as u64),
        ("mul32", 13u64, 4u64),
        ("mul64", 13u64, 4u64),
        ("mul32", 13u64, (-4i32) as u32 as u64),
        ("mul64", 13u64, (-4i64) as u64),
        ("mul64", (-13i64) as u64, 4u64),
        ("mul64", (-13i64) as u64, (-4i64) as u64),
    ];

    for (mnemonic, dst, src) in &cases {
        let src_lo = (*src as u32) as i32;

        // IMM variant: lddw r0, <dst> / lddw r1, <src> / <op> r0, <imm> / exit
        let asm_imm = format!(
            "lddw r0, {}\nlddw r1, {}\n{} r0, {}\nexit",
            format_lddw_arg(*dst),
            format_lddw_arg(*src),
            mnemonic,
            src_lo,
        );
        let ir = sbpf2ir(&asm_imm, vec![], "V0");
        let name = format!("{:03}_v0_{}_imm.ir", idx, mnemonic);
        save_ir(output_dir.join(&name).to_str().unwrap(), &ir);

        // REG variant: lddw r0, <dst> / lddw r1, <src> / <op> r0, r1 / exit
        let asm_reg = format!(
            "lddw r0, {}\nlddw r1, {}\n{} r0, r1\nexit",
            format_lddw_arg(*dst),
            format_lddw_arg(*src),
            mnemonic,
        );
        let ir = sbpf2ir(&asm_reg, vec![], "V0");
        let name = format!("{:03}_v0_{}_reg.ir", idx, mnemonic);
        save_ir(output_dir.join(&name).to_str().unwrap(), &ir);

        *idx += 1;
    }
}

/// Mirrors test_pqr_v2 (execution.rs line 577).
/// V2 uses mov32+hor64 instead of lddw.
fn gen_v2_pqr(output_dir: &Path, idx: &mut usize) {
    let cases: Vec<(&str, u64, u64)> = vec![
        ("uhmul64", 13, 4),
        ("udiv32", 13, 4),
        ("udiv64", 13, 4),
        ("urem32", 13, 4),
        ("urem64", 13, 4),
        ("uhmul64", 13, u32::MAX as u64),
        ("udiv32", 13, u32::MAX as u64),
        ("udiv64", 13, u32::MAX as u64),
        ("urem32", 13, u32::MAX as u64),
        ("urem64", 13, u32::MAX as u64),
        ("uhmul64", u64::MAX, 4),
        ("udiv32", u64::MAX, 4),
        ("udiv64", u64::MAX, 4),
        ("urem32", u64::MAX, 4),
        ("urem64", u64::MAX, 4),
        ("uhmul64", u64::MAX, u32::MAX as u64),
        ("udiv32", u64::MAX, u32::MAX as u64),
        ("udiv64", u64::MAX, u32::MAX as u64),
        ("urem32", u64::MAX, u32::MAX as u64),
        ("urem64", u64::MAX, u32::MAX as u64),
        ("lmul32", 13u64, 4u64),
        ("lmul64", 13u64, 4u64),
        ("shmul64", 13u64, 4u64),
        ("sdiv32", 13u64, 4u64),
        ("sdiv64", 13u64, 4u64),
        ("srem32", 13u64, 4u64),
        ("srem64", 13u64, 4u64),
        ("lmul32", 13u64, (-4i32) as u32 as u64),
        ("lmul64", 13u64, (-4i64) as u64),
        ("shmul64", 13u64, (-4i64) as u64),
        ("sdiv32", 13u64, (-4i32) as u32 as u64),
        ("sdiv64", 13u64, (-4i64) as u64),
        ("srem32", 13u64, (-4i32) as u32 as u64),
        ("srem64", 13u64, (-4i64) as u64),
        ("lmul32", (-13i64) as u64, 4u64),
        ("lmul64", (-13i64) as u64, 4u64),
        ("shmul64", (-13i64) as u64, 4u64),
        ("sdiv32", (-13i64) as u64, 4u64),
        ("sdiv64", (-13i64) as u64, 4u64),
        ("srem32", (-13i64) as u64, 4u64),
        ("srem64", (-13i64) as u64, 4u64),
        ("lmul32", (-13i64) as u64, (-4i32) as u32 as u64),
        ("lmul64", (-13i64) as u64, (-4i64) as u64),
        ("shmul64", (-13i64) as u64, (-4i64) as u64),
        ("sdiv32", (-13i64) as u64, (-4i32) as u32 as u64),
        ("sdiv64", (-13i64) as u64, (-4i64) as u64),
        ("srem32", (-13i64) as u64, (-4i32) as u32 as u64),
        ("srem64", (-13i64) as u64, (-4i64) as u64),
    ];

    for (mnemonic, dst, src) in &cases {
        let dst_lo = *dst as u32;
        let dst_hi = (*dst >> 32) as u32;
        let src_lo = *src as u32;
        let src_hi = (*src >> 32) as u32;
        let imm_val = src_lo as i32;

        // IMM variant
        let asm_imm = format!(
            "add64 r10, 0\nmov32 r0, {}\nhor64 r0, {}\nmov32 r1, {}\nhor64 r1, {}\n{} r0, {}\nexit",
            dst_lo, dst_hi, src_lo, src_hi, mnemonic, imm_val,
        );
        let ir = sbpf2ir(&asm_imm, vec![], "V2");
        let name = format!("{:03}_v2_{}_imm.ir", idx, mnemonic);
        save_ir(output_dir.join(&name).to_str().unwrap(), &ir);

        // REG variant
        let asm_reg = format!(
            "add64 r10, 0\nmov32 r0, {}\nhor64 r0, {}\nmov32 r1, {}\nhor64 r1, {}\n{} r0, r1\nexit",
            dst_lo, dst_hi, src_lo, src_hi, mnemonic,
        );
        let ir = sbpf2ir(&asm_reg, vec![], "V2");
        let name = format!("{:03}_v2_{}_reg.ir", idx, mnemonic);
        save_ir(output_dir.join(&name).to_str().unwrap(), &ir);

        *idx += 1;
    }
}

/// Mirrors test_err_pqr_divide_by_zero (execution.rs line 753).
/// REG-only: r0 = 0, <op> r0, r0 → divide by zero.
fn gen_divzero(output_dir: &Path, idx: &mut usize) {
    let ops = [
        "udiv32", "udiv64", "urem32", "urem64",
        "sdiv32", "sdiv64", "srem32", "srem64",
    ];

    for mnemonic in &ops {
        let asm = format!(
            "add64 r10, 0\nmov32 r0, 0\n{} r0, r0\nexit",
            mnemonic,
        );
        let ir = sbpf2ir(&asm, vec![], "V2");
        let name = format!("{:03}_v2_{}_divzero.ir", idx, mnemonic);
        save_ir(output_dir.join(&name).to_str().unwrap(), &ir);
        *idx += 1;
    }
}

/// Mirrors test_err_pqr_divide_overflow (execution.rs line 793).
/// Signed div/rem of INT_MIN / -1 → overflow.
fn gen_divoverflow(output_dir: &Path, idx: &mut usize) {
    // (mnemonic, shift) — 32-bit uses shift=31, 64-bit uses shift=63
    let ops: Vec<(&str, u32)> = vec![
        ("sdiv32", 31),
        ("sdiv64", 63),
        ("srem32", 31),
        ("srem64", 63),
    ];

    for (mnemonic, shift) in &ops {
        // IMM variant: dst = 1 << shift (INT_MIN), imm = -1
        let asm_imm = format!(
            "add64 r10, 0\nmov64 r0, 1\nlsh64 r0, {}\nmov64 r1, -1\n{} r0, -1\nexit",
            shift, mnemonic,
        );
        let ir = sbpf2ir(&asm_imm, vec![], "V2");
        let name = format!("{:03}_v2_{}_overflow_imm.ir", idx, mnemonic);
        save_ir(output_dir.join(&name).to_str().unwrap(), &ir);

        // REG variant: dst = 1 << shift, src = -1 (in r1)
        let asm_reg = format!(
            "add64 r10, 0\nmov64 r0, 1\nlsh64 r0, {}\nmov64 r1, -1\n{} r0, r1\nexit",
            shift, mnemonic,
        );
        let ir = sbpf2ir(&asm_reg, vec![], "V2");
        let name = format!("{:03}_v2_{}_overflow_reg.ir", idx, mnemonic);
        save_ir(output_dir.join(&name).to_str().unwrap(), &ir);

        *idx += 1;
    }
}

/// Format a u64 as an lddw argument. The assembler accepts signed i64 literals.
fn format_lddw_arg(val: u64) -> String {
    let signed = val as i64;
    format!("{}", signed)
}

/// Generate PQR IR corpus files into `output_dir`. Returns the next free index.
pub fn generate(output_dir: &str) -> usize {
    let path = Path::new(output_dir);
    std::fs::create_dir_all(path).expect("failed to create output directory");

    let mut idx = 0usize;
    gen_v0_pqr(path, &mut idx);
    gen_v2_pqr(path, &mut idx);
    gen_divzero(path, &mut idx);
    gen_divoverflow(path, &mut idx);

    println!("Generated {} PQR IR corpus entries in {}", idx, output_dir);
    idx
}
