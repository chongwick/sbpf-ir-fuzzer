use rand::SeedableRng;
use sbpf_tool::generator::{gen_complex_stress, gen_jit_stress, gen_verifier_stress};
use sbpf_tool::ir::IR;
use sbpf_tool::lowering::{lower, LoweringConfig, StressMode};
use solana_sbpf::{
    ebpf,
    elf::Executable,
    program::{BuiltinProgram, FunctionRegistry, SBPFVersion},
    static_analysis::Analysis,
    vm::ContextObject,
};
use std::env;
use std::sync::Arc;

struct BridgeContext;

impl ContextObject for BridgeContext {
    fn consume(&mut self, _amount: u64) {}

    fn get_remaining(&self) -> u64 {
        0
    }
}

fn built_in_disassemble(program: &[u8]) -> Result<Vec<String>, String> {
    let loader = Arc::new(BuiltinProgram::new_mock());
    let executable = Executable::<BridgeContext>::from_text_bytes(
        program,
        loader,
        SBPFVersion::V4,
        FunctionRegistry::default(),
    )
    .map_err(|e| format!("from_text_bytes failed: {e:?}"))?;
    let analysis =
        Analysis::from_executable(&executable).map_err(|e| format!("analysis failed: {e:?}"))?;
    let mut output = Vec::new();
    analysis
        .disassemble(&mut output)
        .map_err(|e| format!("disassemble failed: {e}"))?;
    let text = String::from_utf8(output).map_err(|e| format!("utf8 decode failed: {e}"))?;
    Ok(text.lines().map(ToOwned::to_owned).collect())
}

fn print_output(ir: &IR, mode: StressMode, max_disasm_lines: usize) {
    let lowered = lower(
        ir,
        &LoweringConfig {
            mode,
            max_stack_depth: 4096,
            max_insn_count: 100_000,
        },
    )
    .expect("lowering should succeed");

    let sbpf_count = lowered.len() / ebpf::INSN_SIZE;
    let disasm_lines = built_in_disassemble(&lowered).expect("built-in disassembly should succeed");

    println!("===IR===");
    println!("{ir:#?}");

    println!("===SUMMARY===");
    println!("regions={}", ir.regions.len());
    println!(
        "ir_instructions={}",
        ir.regions.iter().map(|r| r.instructions.len()).sum::<usize>()
    );
    println!("sbpf_instructions={sbpf_count}");

    println!("===DISASM===");
    let show = disasm_lines.len().min(max_disasm_lines);
    for line in disasm_lines.iter().take(show) {
        println!("{line}");
    }
    if disasm_lines.len() > show {
        println!("... ({} more disassembly lines)", disasm_lines.len() - show);
    }
}

fn parse_or<T: std::str::FromStr>(arg: Option<&String>, default: T) -> T {
    arg.and_then(|s| s.parse::<T>().ok()).unwrap_or(default)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let generator = args.get(1).map(String::as_str).unwrap_or("complex");
    let max_disasm_lines: usize = parse_or(args.get(2), 60_usize);

    match generator {
        "verifier" => {
            let ir = gen_verifier_stress();
            print_output(&ir, StressMode::Verifier, max_disasm_lines);
        }
        "jit" => {
            let ir = gen_jit_stress();
            print_output(&ir, StressMode::Jit, max_disasm_lines);
        }
        "complex" => {
            let min_regions: usize = parse_or(args.get(3), 12_usize);
            let per_region: usize = parse_or(args.get(4), 24_usize);
            let seed: u64 = parse_or(args.get(5), 1337_u64);
            let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
            let ir = gen_complex_stress(&mut rng, min_regions, per_region);
            print_output(&ir, StressMode::Both, max_disasm_lines);
        }
        _ => {
            eprintln!(
                "usage: cargo run -p sbpf-tool --example ir_disasm_bridge -- [verifier|jit|complex] [max_disasm_lines] [min_regions] [per_region] [seed]"
            );
            std::process::exit(2);
        }
    }
}
