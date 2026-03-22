use crate::ir::{sbpf2ir, IrSeq};
use std::path::Path;

fn save_ir(path: &str, ir: &IrSeq) {
    let bytes = bincode::serialize(ir).expect("failed to serialize IR");
    std::fs::write(path, bytes).expect("failed to write IR file");
}

fn emit(dir: &Path, idx: &mut usize, name: &str, asm: &str, memory: Vec<u8>, version: &str) {
    let ir = sbpf2ir(asm, memory, version);
    let filename = format!("{:03}_{}.ir", idx, name);
    save_ir(dir.join(&filename).to_str().unwrap(), &ir);
    *idx += 1;
}

// ---------- ALU ----------

fn gen_mov(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "mov32_imm_1", "\
add64 r10, 0
mov32 r0, 1
exit", vec![], "V0");

    emit(dir, idx, "mov32_imm_neg1", "\
add64 r10, 0
mov32 r0, -1
exit", vec![], "V0");

    emit(dir, idx, "mov32_reg_1", "\
add64 r10, 0
mov32 r1, 1
mov32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "mov32_reg_neg1", "\
add64 r10, 0
mov32 r1, -1
mov32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "mov64_imm_1", "\
add64 r10, 0
mov64 r0, 1
exit", vec![], "V0");

    emit(dir, idx, "mov64_imm_neg1", "\
add64 r10, 0
mov64 r0, -1
exit", vec![], "V0");

    emit(dir, idx, "mov64_reg_1", "\
add64 r10, 0
mov64 r1, 1
mov64 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "mov64_reg_neg1", "\
add64 r10, 0
mov64 r1, -1
mov64 r0, r1
exit", vec![], "V0");
}

fn gen_bounce(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "bounce", "\
add64 r10, 0
mov r0, 1
mov r6, r0
mov r7, r6
mov r8, r7
mov r9, r8
mov r0, r9
exit", vec![], "V0");
}

fn gen_add_sub(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "add32_sub32", "\
add64 r10, 0
mov32 r0, 1
add32 r0, 2
mov32 r1, 5
add32 r0, r1
mov32 r1, 3
sub32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "add64_sub64", "\
add64 r10, 0
mov32 r0, 1
add64 r0, 2
mov32 r1, 5
add64 r0, r1
mov32 r1, 3
sub64 r0, r1
exit", vec![], "V0");
}

fn gen_lmul128(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "lmul128", "\
add64 r10, 0
mov r0, r1
mov r2, 30
mov r3, 0
mov r4, 20
mov r5, 0
mul64 r3, r4
mul64 r5, r2
add64 r5, r3
mov64 r0, r2
rsh64 r0, 0x20
mov64 r3, r4
rsh64 r3, 0x20
mov64 r6, r3
mul64 r6, r0
add64 r5, r6
lsh64 r4, 0x20
rsh64 r4, 0x20
mov64 r6, r4
mul64 r6, r0
lsh64 r2, 0x20
rsh64 r2, 0x20
mul64 r4, r2
mov64 r0, r4
rsh64 r0, 0x20
add64 r0, r6
mov64 r6, r0
rsh64 r6, 0x20
add64 r5, r6
mul64 r3, r2
lsh64 r0, 0x20
rsh64 r0, 0x20
add64 r0, r3
mov64 r2, r0
rsh64 r2, 0x20
add64 r5, r2
stxdw [r1+0x8], r5
lsh64 r0, 0x20
lsh64 r4, 0x20
rsh64 r4, 0x20
or64 r0, r4
stxdw [r1+0x0], r0
exit", vec![0; 16], "V0");
}

// ---------- Logic ----------

fn gen_logic(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "alu32_logic", "\
add64 r10, 0
mov32 r0, 0
mov32 r1, 1
mov32 r2, 2
mov32 r3, 3
mov32 r4, 4
mov32 r5, 5
mov32 r6, 6
mov32 r7, 7
mov32 r8, 8
or32 r0, r5
or32 r0, 0xa0
and32 r0, 0xa3
mov32 r9, 0x91
and32 r0, r9
lsh32 r0, 22
lsh32 r0, r8
rsh32 r0, 19
rsh32 r0, r7
xor32 r0, 0x03
xor32 r0, r2
exit", vec![], "V0");

    emit(dir, idx, "alu64_logic", "\
add64 r10, 0
mov r0, 0
mov r1, 1
mov r2, 2
mov r3, 3
mov r4, 4
mov r5, 5
mov r6, 6
mov r7, 7
mov r8, 8
or r0, r5
or r0, 0xa0
and r0, 0xa3
mov r9, 0x91
and r0, r9
lsh r0, 32
lsh r0, 22
lsh r0, r8
rsh r0, 32
rsh r0, 19
rsh r0, r7
xor r0, 0x03
xor r0, r2
exit", vec![], "V0");
}

// ---------- Shifts ----------

fn gen_shifts(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "arsh32_high_shift", "\
add64 r10, 0
mov r0, 8
lddw r1, 0x100000001
arsh32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "arsh32_imm", "\
add64 r10, 0
mov32 r0, 0xf8
lsh32 r0, 28
arsh32 r0, 16
exit", vec![], "V0");

    emit(dir, idx, "arsh32_reg", "\
add64 r10, 0
mov32 r0, 0xf8
mov32 r1, 16
lsh32 r0, 28
arsh32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "arsh64", "\
add64 r10, 0
mov32 r0, 1
lsh r0, 63
arsh r0, 55
mov32 r1, 5
arsh r0, r1
exit", vec![], "V0");

    emit(dir, idx, "lsh64_reg", "\
add64 r10, 0
mov r0, 0x1
mov r7, 4
lsh r0, r7
exit", vec![], "V0");

    emit(dir, idx, "rsh32_imm", "\
add64 r10, 0
xor r0, r0
add r0, -1
rsh32 r0, 8
exit", vec![], "V0");

    emit(dir, idx, "rsh64_reg", "\
add64 r10, 0
mov r0, 0x10
mov r7, 4
rsh r0, r7
exit", vec![], "V0");
}

// ---------- Byte swaps ----------

fn gen_byteswap(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "be16", "\
add64 r10, 0
ldxh r0, [r1]
be16 r0
exit", vec![0x11, 0x22], "V0");

    emit(dir, idx, "be16_high", "\
add64 r10, 0
ldxdw r0, [r1]
be16 r0
exit", vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88], "V0");

    emit(dir, idx, "be32", "\
add64 r10, 0
ldxw r0, [r1]
be32 r0
exit", vec![0x11, 0x22, 0x33, 0x44], "V0");

    emit(dir, idx, "be32_high", "\
add64 r10, 0
ldxdw r0, [r1]
be32 r0
exit", vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88], "V0");

    emit(dir, idx, "be64", "\
add64 r10, 0
ldxdw r0, [r1]
be64 r0
exit", vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88], "V0");
}

// ---------- Memory instructions (V0 + V4) ----------

fn gen_memory(dir: &Path, idx: &mut usize) {
    for version in ["V0", "V4"] {
        let v = version.to_lowercase();

        emit(dir, idx, &format!("{}_ldxb", v), "\
add64 r10, 0
ldxb r0, [r1+2]
exit", vec![0xaa, 0xbb, 0x11, 0xcc, 0xdd], version);

        emit(dir, idx, &format!("{}_ldxh", v), "\
add64 r10, 0
ldxh r0, [r1+2]
exit", vec![0xaa, 0xbb, 0x11, 0x22, 0xcc, 0xdd], version);

        emit(dir, idx, &format!("{}_ldxw", v), "\
add64 r10, 0
ldxw r0, [r1+2]
exit", vec![0xaa, 0xbb, 0x11, 0x22, 0x33, 0x44, 0xcc, 0xdd], version);

        emit(dir, idx, &format!("{}_ldxdw", v), "\
add64 r10, 0
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xcc, 0xdd],
            version);

        emit(dir, idx, &format!("{}_stb", v), "\
add64 r10, 0
stb [r1+2], 0x11
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0xff, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xcc, 0xdd],
            version);

        emit(dir, idx, &format!("{}_stb_neg", v), "\
add64 r10, 0
stb [r1+2], -1
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xcc, 0xdd],
            version);

        emit(dir, idx, &format!("{}_sth", v), "\
add64 r10, 0
sth [r1+2], 0x2211
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0xff, 0xff, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xcc, 0xdd],
            version);

        emit(dir, idx, &format!("{}_sth_neg", v), "\
add64 r10, 0
sth [r1+2], -1
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xcc, 0xdd],
            version);

        emit(dir, idx, &format!("{}_stw", v), "\
add64 r10, 0
stw [r1+2], 0x44332211
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0xff, 0xff, 0xff, 0xff, 0x55, 0x66, 0x77, 0x88, 0xcc, 0xdd],
            version);

        emit(dir, idx, &format!("{}_stw_neg", v), "\
add64 r10, 0
stw [r1+2], -1
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xcc, 0xdd],
            version);

        emit(dir, idx, &format!("{}_stdw", v), "\
add64 r10, 0
stdw [r1+2], 0x44332211
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xcc, 0xdd],
            version);

        emit(dir, idx, &format!("{}_stdw_neg", v), "\
add64 r10, 0
stdw [r1+2], -1
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xcc, 0xdd],
            version);

        emit(dir, idx, &format!("{}_stxb", v), "\
add64 r10, 0
mov32 r2, 0x11
stxb [r1+2], r2
ldxb r0, [r1+2]
exit", vec![0xaa, 0xbb, 0xff, 0xcc, 0xdd], version);

        emit(dir, idx, &format!("{}_stxh", v), "\
add64 r10, 0
mov32 r2, 0x2211
stxh [r1+2], r2
ldxh r0, [r1+2]
exit", vec![0xaa, 0xbb, 0xff, 0xff, 0xcc, 0xdd], version);

        emit(dir, idx, &format!("{}_stxw", v), "\
add64 r10, 0
mov32 r2, 0x44332211
stxw [r1+2], r2
ldxw r0, [r1+2]
exit", vec![0xaa, 0xbb, 0xff, 0xff, 0xff, 0xff, 0xcc, 0xdd], version);

        emit(dir, idx, &format!("{}_stxdw", v), "\
add64 r10, 0
mov r2, -2005440939
lsh r2, 32
or r2, 0x44332211
stxdw [r1+2], r2
ldxdw r0, [r1+2]
exit",
            vec![0xaa, 0xbb, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xcc, 0xdd],
            version);
    }
}

// ---------- HOR64 (V2) ----------

fn gen_hor64(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "v2_hor64", "\
add64 r10, 0
hor64 r0, 0x10203040
hor64 r0, 0x01020304
exit", vec![], "V2");
}

// ---------- LDX/STX variants ----------

fn gen_ldx_stx_variants(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "ldxh_same_reg", "\
add64 r10, 0
mov r0, r1
sth [r0], 0x1234
ldxh r0, [r0]
exit", vec![0xff, 0xff], "V0");

    emit(dir, idx, "err_ldxdw_oob", "\
add64 r10, 0
ldxdw r0, [r1+6]
exit",
        vec![0xaa, 0xbb, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xcc, 0xdd],
        "V0");

    emit(dir, idx, "err_ldxdw_nomem", "\
add64 r10, 0
ldxdw r0, [r1+6]
exit", vec![], "V0");

    emit(dir, idx, "ldxb_all", "\
add64 r10, 0
mov r0, r1
ldxb r9, [r0+0]
lsh r9, 0
ldxb r8, [r0+1]
lsh r8, 4
ldxb r7, [r0+2]
lsh r7, 8
ldxb r6, [r0+3]
lsh r6, 12
ldxb r5, [r0+4]
lsh r5, 16
ldxb r4, [r0+5]
lsh r4, 20
ldxb r3, [r0+6]
lsh r3, 24
ldxb r2, [r0+7]
lsh r2, 28
ldxb r1, [r0+8]
lsh r1, 32
ldxb r0, [r0+9]
lsh r0, 36
or r0, r1
or r0, r2
or r0, r3
or r0, r4
or r0, r5
or r0, r6
or r0, r7
or r0, r8
or r0, r9
exit",
        vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09],
        "V0");

    emit(dir, idx, "ldxh_all", "\
add64 r10, 0
mov r0, r1
ldxh r9, [r0+0]
be16 r9
lsh r9, 0
ldxh r8, [r0+2]
be16 r8
lsh r8, 4
ldxh r7, [r0+4]
be16 r7
lsh r7, 8
ldxh r6, [r0+6]
be16 r6
lsh r6, 12
ldxh r5, [r0+8]
be16 r5
lsh r5, 16
ldxh r4, [r0+10]
be16 r4
lsh r4, 20
ldxh r3, [r0+12]
be16 r3
lsh r3, 24
ldxh r2, [r0+14]
be16 r2
lsh r2, 28
ldxh r1, [r0+16]
be16 r1
lsh r1, 32
ldxh r0, [r0+18]
be16 r0
lsh r0, 36
or r0, r1
or r0, r2
or r0, r3
or r0, r4
or r0, r5
or r0, r6
or r0, r7
or r0, r8
or r0, r9
exit",
        vec![
            0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03,
            0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00, 0x07,
            0x00, 0x08, 0x00, 0x09,
        ],
        "V0");

    emit(dir, idx, "ldxh_all2", "\
add64 r10, 0
mov r0, r1
ldxh r9, [r0+0]
be16 r9
ldxh r8, [r0+2]
be16 r8
ldxh r7, [r0+4]
be16 r7
ldxh r6, [r0+6]
be16 r6
ldxh r5, [r0+8]
be16 r5
ldxh r4, [r0+10]
be16 r4
ldxh r3, [r0+12]
be16 r3
ldxh r2, [r0+14]
be16 r2
ldxh r1, [r0+16]
be16 r1
ldxh r0, [r0+18]
be16 r0
or r0, r1
or r0, r2
or r0, r3
or r0, r4
or r0, r5
or r0, r6
or r0, r7
or r0, r8
or r0, r9
exit",
        vec![
            0x00, 0x01, 0x00, 0x02, 0x00, 0x04, 0x00, 0x08,
            0x00, 0x10, 0x00, 0x20, 0x00, 0x40, 0x00, 0x80,
            0x01, 0x00, 0x02, 0x00,
        ],
        "V0");

    emit(dir, idx, "ldxw_all", "\
add64 r10, 0
mov r0, r1
ldxw r9, [r0+0]
be32 r9
ldxw r8, [r0+4]
be32 r8
ldxw r7, [r0+8]
be32 r7
ldxw r6, [r0+12]
be32 r6
ldxw r5, [r0+16]
be32 r5
ldxw r4, [r0+20]
be32 r4
ldxw r3, [r0+24]
be32 r3
ldxw r2, [r0+28]
be32 r2
ldxw r1, [r0+32]
be32 r1
ldxw r0, [r0+36]
be32 r0
or r0, r1
or r0, r2
or r0, r3
or r0, r4
or r0, r5
or r0, r6
or r0, r7
or r0, r8
or r0, r9
exit",
        vec![
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
        ],
        "V0");

    emit(dir, idx, "stxb_all", "\
add64 r10, 0
mov r0, 0xf0
mov r2, 0xf2
mov r3, 0xf3
mov r4, 0xf4
mov r5, 0xf5
mov r6, 0xf6
mov r7, 0xf7
mov r8, 0xf8
stxb [r1], r0
stxb [r1+1], r2
stxb [r1+2], r3
stxb [r1+3], r4
stxb [r1+4], r5
stxb [r1+5], r6
stxb [r1+6], r7
stxb [r1+7], r8
ldxdw r0, [r1]
be64 r0
exit",
        vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        "V0");

    emit(dir, idx, "stxb_all2", "\
add64 r10, 0
mov r0, r1
mov r1, 0xf1
mov r9, 0xf9
stxb [r0], r1
stxb [r0+1], r9
ldxh r0, [r0]
be16 r0
exit", vec![0xff, 0xff], "V0");

    emit(dir, idx, "stxb_chain", "\
add64 r10, 0
mov r0, r1
ldxb r9, [r0+0]
stxb [r0+1], r9
ldxb r8, [r0+1]
stxb [r0+2], r8
ldxb r7, [r0+2]
stxb [r0+3], r7
ldxb r6, [r0+3]
stxb [r0+4], r6
ldxb r5, [r0+4]
stxb [r0+5], r5
ldxb r4, [r0+5]
stxb [r0+6], r4
ldxb r3, [r0+6]
stxb [r0+7], r3
ldxb r2, [r0+7]
stxb [r0+8], r2
ldxb r1, [r0+8]
stxb [r0+9], r1
ldxb r0, [r0+9]
exit",
        vec![0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        "V0");
}

// ---------- Exits/Jumps ----------

fn gen_exits_jumps(dir: &Path, idx: &mut usize) {
    for version in ["V0", "V4"] {
        let v = version.to_lowercase();

        emit(dir, idx, &format!("{}_exit_capped", v), "\
add64 r10, 0
exit", vec![], version);

        emit(dir, idx, &format!("{}_exit_without_value", v), "\
add64 r10, 0
exit", vec![], version);

        emit(dir, idx, &format!("{}_exit", v), "\
add64 r10, 0
mov r0, 0
exit", vec![], version);

        emit(dir, idx, &format!("{}_early_exit", v), "\
add64 r10, 0
mov r0, 3
exit
mov r0, 4
exit", vec![], version);
    }

    emit(dir, idx, "ja", "\
add64 r10, 0
mov r0, 1
ja +1
mov r0, 2
exit", vec![], "V0");
}

// ---------- Stack/Calls ----------

fn gen_stack_calls(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "stack1", "\
add64 r10, 64
mov r1, 51
stdw [r10-16], 0xab
stdw [r10-8], 0xcd
and r1, 1
lsh r1, 3
mov r2, r10
add r2, r1
ldxdw r0, [r2-16]
exit", vec![], "V0");

    for version in ["V0", "V4"] {
        let v = version.to_lowercase();

        emit(dir, idx, &format!("{}_entrypoint_exit", v), "\
entrypoint:
add64 r10, 0
call function_foo
mov r0, 42
exit
function_foo:
add64 r10, 0
mov r0, 12
exit", vec![], version);

        emit(dir, idx, &format!("{}_stack_call_depth_sibling", v), "\
add64 r10, 0
call function_foo
call function_foo
exit
function_foo:
add64 r10, 0
exit", vec![], version);

        emit(dir, idx, &format!("{}_stack_call_depth_nested", v), "\
entrypoint:
add64 r10, 0
call function_foo
exit
function_foo:
add64 r10, 0
call function_bar
exit
function_bar:
add64 r10, 0
exit", vec![], version);
    }

    emit(dir, idx, "bpf_to_bpf_scratch_registers", "\
add64 r10, 0
mov64 r6, 0x11
mov64 r7, 0x22
mov64 r8, 0x44
mov64 r9, 0x88
call function_foo
mov64 r0, r6
add64 r0, r7
add64 r0, r8
add64 r0, r9
exit
function_foo:
add64 r10, 0
mov64 r6, 0x00
mov64 r7, 0x00
mov64 r8, 0x00
mov64 r9, 0x00
exit", vec![], "V0");

    emit(dir, idx, "callx", "\
add64 r10, 0
mov64 r0, 0x0
mov64 r8, 0x1
lsh64 r8, 0x20
or64 r8, 0x38
callx r8
exit
function_foo:
add64 r10, 0
mov64 r0, 0x2A
exit", vec![], "V0");

    emit(dir, idx, "v0_err_callx_oob_low", "\
add64 r10, 0
mov64 r0, 0x3
callx r0
exit", vec![], "V0");

    emit(dir, idx, "err_callx_oob_high", "\
add64 r10, 0
lddw r0, 0x200000000
callx r0
exit", vec![], "V0");

    emit(dir, idx, "err_callx_oob_max", "\
add64 r10, 0
lddw r0, 0xFFFFFFFFFFFFFFF8
callx r0
exit", vec![], "V0");
}

// ---------- Instruction Meter ----------

fn gen_instruction_meter(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "tight_infinite_loop_cond", "\
add64 r10, 0
jsge r0, r0, -1
exit", vec![], "V0");

    emit(dir, idx, "tight_infinite_loop_uncond", "\
add64 r10, 0
ja -1
exit", vec![], "V0");

    emit(dir, idx, "tight_infinite_recursion", "\
entrypoint:
add64 r10, 0
mov64 r3, 0x41414141
call entrypoint
exit", vec![], "V0");

    emit(dir, idx, "tight_infinite_recursion_callx", "\
add64 r10, 0
mov64 r8, 0x1
lsh64 r8, 0x20
or64 r8, 0x30
call function_foo
exit
function_foo:
add64 r10, 0
callx r8
exit", vec![], "V0");

    emit(dir, idx, "err_non_terminate_capped_short", "\
add64 r10, 0
mov64 r6, 0x0
mov64 r1, 0x0
mov64 r2, 0x0
mov64 r3, 0x0
mov64 r4, 0x0
mov64 r5, r6
add64 r6, 0x1
ja -0x8
exit", vec![], "V0");

    emit(dir, idx, "err_non_terminate_capped_long", "\
add64 r10, 0
mov64 r6, 0x0
mov64 r1, 0x0
mov64 r2, 0x0
mov64 r3, 0x0
mov64 r4, 0x0
mov64 r5, r6
add64 r6, 0x1
ja -0x8
exit", vec![], "V0");

    emit(dir, idx, "err_capped_before_div", "\
add64 r10, 0
mov64 r1, 0x0
mov64 r2, 0x0
div64 r1, r2
mov64 r0, 0x0
exit", vec![], "V0");

    emit(dir, idx, "err_capped_before_callx", "\
add64 r10, 0
mov64 r1, 0x0
mov64 r2, 0x0
callx r2
mov64 r0, 0x0
exit", vec![], "V0");

    emit(dir, idx, "err_exit_capped_1", "\
add64 r10, 0
mov64 r0, 0x1
lsh64 r0, 0x20
or64 r0, 0x30
callx r0
exit
function_foo:
add64 r10, 0
exit", vec![], "V0");

    emit(dir, idx, "err_exit_capped_2", "\
add64 r10, 0
mov64 r0, 0x1
lsh64 r0, 0x20
or64 r0, 0x30
callx r0
exit
function_foo:
add64 r10, 0
mov r0, r0
exit", vec![], "V0");

    emit(dir, idx, "err_exit_capped_3", "\
add64 r10, 0
call function_foo
exit
function_foo:
add64 r10, 0
mov r0, r0
exit", vec![], "V0");
}

// ---------- Far jumps ----------

fn gen_far_jumps(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "far_jumps", "\
add64 r10, 0
call function_c
exit
function_a:
add64 r10, 0
exit
function_b:
.fill 1024, 0x0F
exit
function_c:
add64 r10, 0
mov64 r8, 0x1
lsh64 r8, 0x20
or64 r8, 0x18
callx r8
exit", vec![], "V0");
}

// ---------- Programs ----------

fn gen_programs(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "lmul_loop", "\
add64 r10, 0
mov r0, 0x7
add r1, 0xa
lsh r1, 0x20
rsh r1, 0x20
jeq r1, 0x0, +4
mov r0, 0x7
mul r0, 0x7
add r1, -1
jne r1, 0x0, -3
exit", vec![], "V0");

    emit(dir, idx, "prime", "\
add64 r10, 0
mov r1, 67
mov r0, 0x1
mov r2, 0x2
jgt r1, 0x2, +4
ja +10
add r2, 0x1
mov r0, 0x1
jge r2, r1, +7
mov r3, r1
div r3, r2
mul r3, r2
mov r4, r1
sub r4, r3
mov r0, 0x0
jne r4, 0x0, -10
exit", vec![], "V0");

    emit(dir, idx, "subnet", "\
add64 r10, 0
mov r2, 0xe
ldxh r3, [r1+12]
jne r3, 0x81, +2
mov r2, 0x12
ldxh r3, [r1+16]
and r3, 0xffff
jne r3, 0x8, +5
add r1, r2
mov r0, 0x1
ldxw r1, [r1+16]
and r1, 0xffffff
jeq r1, 0x1a8c0, +1
mov r0, 0x0
exit",
        vec![
            0x00, 0x00, 0xc0, 0x9f, 0xa0, 0x97, 0x00, 0xa0,
            0xcc, 0x3b, 0xbf, 0xfa, 0x08, 0x00, 0x45, 0x10,
            0x00, 0x3c, 0x46, 0x3c, 0x40, 0x00, 0x40, 0x06,
            0x73, 0x1c, 0xc0, 0xa8, 0x01, 0x02, 0xc0, 0xa8,
            0x01, 0x01, 0x06, 0x0e, 0x00, 0x17, 0x99, 0xc5,
            0xa0, 0xec, 0x00, 0x00, 0x00, 0x00, 0xa0, 0x02,
            0x7d, 0x78, 0xe0, 0xa3, 0x00, 0x00, 0x02, 0x04,
            0x05, 0xb4, 0x04, 0x02, 0x08, 0x0a, 0x00, 0x9c,
            0x27, 0x24, 0x00, 0x00, 0x00, 0x00, 0x01, 0x03,
            0x03, 0x00,
        ],
        "V0");

    // TCP port 80 tests
    let prog_tcp_port_80 = "\
add64 r10, 0
ldxb r2, [r1+0xc]
ldxb r3, [r1+0xd]
lsh64 r3, 0x8
or64 r3, r2
mov64 r0, 0x0
jne r3, 0x8, +0xc
ldxb r2, [r1+0x17]
jne r2, 0x6, +0xa
ldxb r2, [r1+0xe]
add64 r1, 0xe
and64 r2, 0xf
lsh64 r2, 0x2
add64 r1, r2
ldxh r2, [r1+0x2]
jeq r2, 0x5000, +0x2
ldxh r1, [r1+0x0]
jne r1, 0x5000, +0x1
mov64 r0, 0x1
exit";

    let tcp_match_mem = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x06,
        0x07, 0x08, 0x09, 0x0a, 0x08, 0x00, 0x45, 0x00,
        0x00, 0x56, 0x00, 0x01, 0x00, 0x00, 0x40, 0x06,
        0xf9, 0x4d, 0xc0, 0xa8, 0x00, 0x01, 0xc0, 0xa8,
        0x00, 0x02, 0x27, 0x10, 0x00, 0x50, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x02,
        0x20, 0x00, 0xc5, 0x18, 0x00, 0x00, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44,
    ];

    emit(dir, idx, "tcp_port80_match", prog_tcp_port_80, tcp_match_mem, "V0");

    let tcp_nomatch_mem = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x06,
        0x07, 0x08, 0x09, 0x0a, 0x08, 0x00, 0x45, 0x00,
        0x00, 0x56, 0x00, 0x01, 0x00, 0x00, 0x40, 0x06,
        0xf9, 0x4d, 0xc0, 0xa8, 0x00, 0x01, 0xc0, 0xa8,
        0x00, 0x02, 0x00, 0x16, 0x27, 0x10, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x51, 0x02,
        0x20, 0x00, 0xc5, 0x18, 0x00, 0x00, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44,
    ];

    emit(dir, idx, "tcp_port80_nomatch", prog_tcp_port_80, tcp_nomatch_mem, "V0");

    let tcp_nomatch_ethertype_mem = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x06,
        0x07, 0x08, 0x09, 0x0a, 0x08, 0x01, 0x45, 0x00,
        0x00, 0x56, 0x00, 0x01, 0x00, 0x00, 0x40, 0x06,
        0xf9, 0x4d, 0xc0, 0xa8, 0x00, 0x01, 0xc0, 0xa8,
        0x00, 0x02, 0x27, 0x10, 0x00, 0x50, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x02,
        0x20, 0x00, 0xc5, 0x18, 0x00, 0x00, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44,
    ];

    emit(dir, idx, "tcp_port80_nomatch_ethertype", prog_tcp_port_80, tcp_nomatch_ethertype_mem, "V0");

    let tcp_nomatch_proto_mem = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x00, 0x06,
        0x07, 0x08, 0x09, 0x0a, 0x08, 0x00, 0x45, 0x00,
        0x00, 0x56, 0x00, 0x01, 0x00, 0x00, 0x40, 0x11,
        0xf9, 0x4d, 0xc0, 0xa8, 0x00, 0x01, 0xc0, 0xa8,
        0x00, 0x02, 0x27, 0x10, 0x00, 0x50, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x02,
        0x20, 0x00, 0xc5, 0x18, 0x00, 0x00, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44,
    ];

    emit(dir, idx, "tcp_port80_nomatch_proto", prog_tcp_port_80, tcp_nomatch_proto_mem, "V0");

    // TCP SACK tests
    let tcp_sack_asm = "\
add64 r10, 0
ldxb r2, [r1+12]
ldxb r3, [r1+13]
lsh r3, 0x8
or r3, r2
mov r0, 0x0
jne r3, 0x8, +37
ldxb r2, [r1+23]
jne r2, 0x6, +35
ldxb r2, [r1+14]
add r1, 0xe
and r2, 0xf
lsh r2, 0x2
add r1, r2
mov r0, 0x0
ldxh r4, [r1+12]
add r1, 0x14
rsh r4, 0x2
and r4, 0x3c
mov r2, r4
add r2, -20
mov r5, 0x15
mov r3, 0x0
jgt r5, r4, +20
mov r5, r3
lsh r5, 0x20
arsh r5, 0x20
mov r4, r1
add r4, r5
ldxb r5, [r4]
jeq r5, 0x1, +4
jeq r5, 0x0, +12
mov r6, r3
jeq r5, 0x5, +9
ja +2
add r3, 0x1
mov r6, r3
ldxb r3, [r4+1]
add r3, r6
lsh r3, 0x20
arsh r3, 0x20
jsgt r2, r3, -18
ja +1
mov r0, 0x1
exit";

    let tcp_sack_match_mem = vec![
        0x00, 0x26, 0x62, 0x2f, 0x47, 0x87, 0x00, 0x1d,
        0x60, 0xb3, 0x01, 0x84, 0x08, 0x00, 0x45, 0x00,
        0x00, 0x40, 0xa8, 0xde, 0x40, 0x00, 0x40, 0x06,
        0x9d, 0x58, 0xc0, 0xa8, 0x01, 0x03, 0x3f, 0x74,
        0xf3, 0x61, 0xe5, 0xc0, 0x00, 0x50, 0xe5, 0x94,
        0x3f, 0x77, 0xa3, 0xc4, 0xc4, 0x80, 0xb0, 0x10,
        0x01, 0x3e, 0x34, 0xb6, 0x00, 0x00, 0x01, 0x01,
        0x08, 0x0a, 0x00, 0x17, 0x95, 0x6f, 0x8d, 0x9d,
        0x9e, 0x27, 0x01, 0x01, 0x05, 0x0a, 0xa3, 0xc4,
        0xca, 0x28, 0xa3, 0xc4, 0xcf, 0xd0,
    ];

    emit(dir, idx, "tcp_sack_match", tcp_sack_asm, tcp_sack_match_mem, "V0");

    let tcp_sack_nomatch_mem = vec![
        0x00, 0x26, 0x62, 0x2f, 0x47, 0x87, 0x00, 0x1d,
        0x60, 0xb3, 0x01, 0x84, 0x08, 0x00, 0x45, 0x00,
        0x00, 0x40, 0xa8, 0xde, 0x40, 0x00, 0x40, 0x06,
        0x9d, 0x58, 0xc0, 0xa8, 0x01, 0x03, 0x3f, 0x74,
        0xf3, 0x61, 0xe5, 0xc0, 0x00, 0x50, 0xe5, 0x94,
        0x3f, 0x77, 0xa3, 0xc4, 0xc4, 0x80, 0x80, 0x10,
        0x01, 0x3e, 0x34, 0xb6, 0x00, 0x00, 0x01, 0x01,
        0x08, 0x0a, 0x00, 0x17, 0x95, 0x6f, 0x8d, 0x9d,
        0x9e, 0x27,
    ];

    emit(dir, idx, "tcp_sack_nomatch", tcp_sack_asm, tcp_sack_nomatch_mem, "V0");
}

// ---------- Callx/Other ----------

fn gen_callx_other(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "callx_unsupported", "\
add64 r10, 0
sub32 r7, r1
add64 r5, -8
add64 r7, 0
callx r5
exit", vec![], "V0");

    emit(dir, idx, "capped_after_callx", "\
add64 r10, 0
mov64 r0, 0x0
mov64 r8, 0x1
lsh64 r8, 0x20
or64 r8, 0x38
callx r8
exit
function_foo:
add64 r10, 0
mov64 r0, 0x2A
exit", vec![], "V0");

    emit(dir, idx, "exit_to_nothing_is_capped", "\
a:
exit
entrypoint:
call -2", vec![], "V0");
}

// ---------- V0-Specific ----------

fn gen_v0_specific(dir: &Path, idx: &mut usize) {
    emit(dir, idx, "v0_err_fixed_stack_oob", "\
add64 r10, 0
stb [r10-0x4000], 0
exit", vec![], "V0");

    emit(dir, idx, "v0_execution_overrun_1", "\
add64 r10, 0
add r1, 0", vec![], "V0");

    emit(dir, idx, "v0_execution_overrun_2", "\
add64 r10, 0
add r1, 0", vec![], "V0");

    emit(dir, idx, "v0_execution_overrun_3", "\
add64 r10, 0
add r1, 0", vec![], "V0");

    emit(dir, idx, "v0_mov32_reg_truncating", "\
add64 r10, 0
mov64 r1, -1
mov32 r0, r1
exit", vec![], "V0");

    // lddw tests
    emit(dir, idx, "v0_lddw_overrun", "\
add64 r10, 0
lddw r0, 0x1122334455667788", vec![], "V0");

    emit(dir, idx, "v0_lddw_basic", "\
add64 r10, 0
lddw r0, 0x1122334455667788
exit", vec![], "V0");

    emit(dir, idx, "v0_lddw_80000000", "\
add64 r10, 0
lddw r0, 0x0000000080000000
exit", vec![], "V0");

    emit(dir, idx, "v0_lddw_ja_over", "\
add64 r10, 0
mov r0, 0
mov r1, 0
mov r2, 0
lddw r0, 0x1
ja +2
lddw r1, 0x1
lddw r2, 0x1
add r1, r2
add r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_lddw_callx_capped1", "\
add64 r10, 0
mov64 r8, 0x1
lsh64 r8, 0x20
or64 r8, 0x30
callx r8
lddw r0, 0x1122334455667788
exit", vec![], "V0");

    emit(dir, idx, "v0_lddw_callx_capped2", "\
add64 r10, 0
mov64 r8, 0x1
lsh64 r8, 0x20
or64 r8, 0x30
callx r8
lddw r0, 0x1122334455667788
exit", vec![], "V0");

    emit(dir, idx, "v0_lddw_callx_unsupported1", "\
add64 r10, 0
mov64 r1, 0x1
lsh64 r1, 0x20
or64 r1, 0x40
callx r1
mov r0, r0
mov r0, r0
lddw r0, 0x1122334455667788
exit", vec![], "V0");

    emit(dir, idx, "v0_lddw_callx_unsupported2", "\
add64 r10, 0
lddw r1, 0x100000040
callx r1
mov r0, r0
mov r0, r0
exit
lddw r0, 0x1122334455667788
exit", vec![], "V0");

    emit(dir, idx, "v0_lddw_exit_capped", "\
add64 r10, 0
mov r0, 0
lddw r1, 0x1
mov r2, 0
exit", vec![], "V0");

    // le tests (V0 only)
    emit(dir, idx, "v0_le16", "\
add64 r10, 0
ldxh r0, [r1]
le16 r0
exit", vec![0x22, 0x11], "V0");

    emit(dir, idx, "v0_le16_high", "\
add64 r10, 0
ldxdw r0, [r1]
le16 r0
exit", vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88], "V0");

    emit(dir, idx, "v0_le32", "\
add64 r10, 0
ldxw r0, [r1]
le32 r0
exit", vec![0x44, 0x33, 0x22, 0x11], "V0");

    emit(dir, idx, "v0_le32_high", "\
add64 r10, 0
ldxdw r0, [r1]
le32 r0
exit", vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88], "V0");

    emit(dir, idx, "v0_le64", "\
add64 r10, 0
ldxdw r0, [r1]
le64 r0
exit", vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11], "V0");

    // neg tests (V0 only)
    emit(dir, idx, "v0_neg32", "\
add64 r10, 0
mov32 r0, 2
neg32 r0
exit", vec![], "V0");

    emit(dir, idx, "v0_neg64", "\
add64 r10, 0
mov r0, 2
neg r0
exit", vec![], "V0");

    emit(dir, idx, "v0_sub32_as_neg", "\
add64 r10, 0
mov32 r0, 3
sub32 r0, 1
exit", vec![], "V0");

    emit(dir, idx, "v0_sub64_as_neg", "\
add64 r10, 0
mov r0, 3
sub r0, 1
exit", vec![], "V0");

    // callx_imm (V0 only, no add64 r10,0 in function_foo)
    emit(dir, idx, "v0_callx_imm", "\
add64 r10, 0
mov64 r0, 0x0
mov64 r8, 0x1
lsh64 r8, 0x20
or64 r8, 0x38
callx r8
exit
function_foo:
mov64 r0, 0x2A
exit", vec![], "V0");

    // mul tests (V0 only)
    emit(dir, idx, "v0_mul32_imm", "\
add64 r10, 0
mov r0, 3
mul32 r0, 4
exit", vec![], "V0");

    emit(dir, idx, "v0_mul32_reg", "\
add64 r10, 0
mov r0, 3
mov r1, 4
mul32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_mul32_overflow", "\
add64 r10, 0
mov r0, 0x40000001
mov r1, 4
mul32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_mul64_imm", "\
add64 r10, 0
mov r0, 0x40000001
mul r0, 4
exit", vec![], "V0");

    emit(dir, idx, "v0_mul64_reg", "\
add64 r10, 0
mov r0, 0x40000001
mov r1, 4
mul r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_mul32_neg", "\
add64 r10, 0
mov r0, -1
mul32 r0, 4
exit", vec![], "V0");

    // div tests (V0 only)
    emit(dir, idx, "v0_div32_reg", "\
add64 r10, 0
mov r0, 12
lddw r1, 0x100000004
div32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_div32_imm", "\
add64 r10, 0
lddw r0, 0x10000000c
div32 r0, 4
exit", vec![], "V0");

    emit(dir, idx, "v0_div32_reg2", "\
add64 r10, 0
lddw r0, 0x10000000c
mov r1, 4
div32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_div64_imm", "\
add64 r10, 0
mov r0, 0xc
lsh r0, 32
div r0, 4
exit", vec![], "V0");

    emit(dir, idx, "v0_div64_reg", "\
add64 r10, 0
mov r0, 0xc
lsh r0, 32
mov r1, 4
div r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_div64_by_zero", "\
add64 r10, 0
mov32 r0, 1
mov32 r1, 0
div r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_div32_by_zero", "\
add64 r10, 0
mov32 r0, 1
mov32 r1, 0
div32 r0, r1
exit", vec![], "V0");

    // mod tests (V0 only)
    emit(dir, idx, "v0_mod32_imm_reg", "\
add64 r10, 0
mov32 r0, 5748
mod32 r0, 92
mov32 r1, 13
mod32 r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_mod32_lddw", "\
add64 r10, 0
lddw r0, 0x100000003
mod32 r0, 3
exit", vec![], "V0");

    emit(dir, idx, "v0_mod64_complex", "\
add64 r10, 0
mov32 r0, -1316649930
lsh r0, 32
or r0, 0x100dc5c8
mov32 r1, 0xdde263e
lsh r1, 32
or r1, 0x3cbef7f3
mod r0, r1
mod r0, 0x658f1778
exit", vec![], "V0");

    emit(dir, idx, "v0_mod64_by_zero", "\
add64 r10, 0
mov32 r0, 1
mov32 r1, 0
mod r0, r1
exit", vec![], "V0");

    emit(dir, idx, "v0_mod32_by_zero", "\
add64 r10, 0
mov32 r0, 1
mov32 r1, 0
mod32 r0, r1
exit", vec![], "V0");

    // Stack gaps
    emit(dir, idx, "v0_stack_gaps", "\
stw [r10 + 8], 77
call function_foo
exit
function_foo:
ldxdw r0, [r10 - 4088]
exit", vec![], "V0");

    emit(dir, idx, "v3_stack_gaps", "\
stw [r10 + 8], 77
call function_foo
exit
function_foo:
ldxdw r0, [r10 - 4088]
exit", vec![], "V3");

    // err_call_unresolved (V0 only, uses syscall which won't resolve)
    emit(dir, idx, "v0_err_call_unresolved", "\
add64 r10, 0
mov r1, 1
mov r2, 2
mov r3, 3
mov r4, 4
mov r5, 5
syscall Unresolved
mov64 r0, 0x0
exit", vec![], "V0");
}

/// Generate non-PQR IR corpus files. `start_idx` continues numbering from PQR.
/// Returns the next free index.
pub fn generate(output_dir: &str, start_idx: usize) -> usize {
    let path = Path::new(output_dir);
    std::fs::create_dir_all(path).expect("failed to create output directory");

    let mut idx = start_idx;
    let start = idx;

    gen_mov(path, &mut idx);
    gen_bounce(path, &mut idx);
    gen_add_sub(path, &mut idx);
    gen_lmul128(path, &mut idx);
    gen_logic(path, &mut idx);
    gen_shifts(path, &mut idx);
    gen_byteswap(path, &mut idx);
    gen_memory(path, &mut idx);
    gen_hor64(path, &mut idx);
    gen_ldx_stx_variants(path, &mut idx);
    gen_exits_jumps(path, &mut idx);
    gen_stack_calls(path, &mut idx);
    gen_instruction_meter(path, &mut idx);
    gen_far_jumps(path, &mut idx);
    gen_programs(path, &mut idx);
    gen_callx_other(path, &mut idx);
    gen_v0_specific(path, &mut idx);

    println!("Generated {} general IR corpus entries in {}", idx - start, output_dir);
    idx
}
