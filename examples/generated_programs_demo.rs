use rand::SeedableRng;
use sbpf_tool::generator::{gen_complex_stress, gen_jit_stress, gen_verifier_stress};
use sbpf_tool::ir::IR;
use sbpf_tool::lowering::{lower, LoweringConfig, StressMode};
use solana_sbpf::ebpf;

fn branch_target(pc: usize, off: i16) -> usize {
    (pc as isize + off as isize + 1) as usize
}

fn disasm_insn(insn: &ebpf::Insn) -> String {
    match insn.opc {
        ebpf::MOV64_IMM => format!("mov64 r{}, {}", insn.dst, insn.imm),
        ebpf::MOV64_REG => format!("mov64 r{}, r{}", insn.dst, insn.src),
        ebpf::ADD64_IMM => format!("add64 r{}, {}", insn.dst, insn.imm),
        ebpf::ADD64_REG => format!("add64 r{}, r{}", insn.dst, insn.src),
        ebpf::SUB64_IMM => format!("sub64 r{}, {}", insn.dst, insn.imm),
        ebpf::XOR64_IMM => format!("xor64 r{}, {}", insn.dst, insn.imm),
        ebpf::ST_DW_REG => format!("stxdw [r{}{:+}], r{}", insn.dst, insn.off, insn.src),
        ebpf::ST_DW_IMM => format!("stdw [r{}{:+}], {}", insn.dst, insn.off, insn.imm),
        ebpf::LD_DW_REG => format!("ldxdw r{}, [r{}{:+}]", insn.dst, insn.src, insn.off),
        ebpf::JGT64_IMM => format!(
            "jgt r{}, {}, -> pc{}",
            insn.dst,
            insn.imm,
            branch_target(insn.ptr, insn.off)
        ),
        ebpf::JSGT64_IMM => format!(
            "jsgt r{}, {}, -> pc{}",
            insn.dst,
            insn.imm,
            branch_target(insn.ptr, insn.off)
        ),
        ebpf::JA => format!("ja -> pc{}", branch_target(insn.ptr, insn.off)),
        ebpf::CALL_IMM => format!("call/syscall {}", insn.imm),
        ebpf::EXIT => "exit".to_string(),
        _ => format!(
            "opc=0x{:02x} dst=r{} src=r{} off={} imm={}",
            insn.opc, insn.dst, insn.src, insn.off, insn.imm
        ),
    }
}

fn summarize(name: &str, ir: &IR, mode: StressMode) {
    let region_count = ir.regions.len();
    let ir_insn_count: usize = ir.regions.iter().map(|r| r.instructions.len()).sum();

    let lowered = lower(
        ir,
        &LoweringConfig {
            mode,
            max_stack_depth: 4096,
            max_insn_count: 50_000,
        },
    )
    .expect("lowering should succeed");

    let sbpf_count = lowered.len() / ebpf::INSN_SIZE;

    println!("=== {name} ===");
    println!("regions: {region_count}");
    println!("ir instructions: {ir_insn_count}");
    println!("lowered sBPF instructions: {sbpf_count}");
    println!("disassembly (first 25 instructions):");

    let show = sbpf_count.min(25);
    for pc in 0..show {
        let insn = ebpf::get_insn(&lowered, pc);
        println!("  {pc:>4}: {}", disasm_insn(&insn));
    }
    if sbpf_count > show {
        println!("  ... ({} more instructions)", sbpf_count - show);
    }
    println!();
}

fn main() {
    let verifier = gen_verifier_stress();
    let jit = gen_jit_stress();
    let mut rng = rand::rngs::SmallRng::seed_from_u64(1337);
    let complex = gen_complex_stress(&mut rng, 12, 24);

    summarize("Verifier Stress", &verifier, StressMode::Verifier);
    summarize("JIT Stress", &jit, StressMode::Jit);
    summarize("Complex Stress", &complex, StressMode::Both);
}
