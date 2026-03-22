mod executor;
mod gen_corpus;
mod gen_pqr;
mod ir;
mod mutator;

use executor::{parse_sbpf_version, run_diff, run_diff_ir, triage_ir};
use ir::{sbpf2ir, IrSeq};
use mutator::mutate;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Deserialize;
use solana_sbpf::vm::Config;

// --- File I/O ---

#[derive(Deserialize)]
struct InputMeta {
    version: String,
    memory: Vec<u8>,
    asm: String,
}

#[derive(Deserialize)]
struct ExecMeta {
    version: String,
    #[serde(default)]
    memory: Vec<u8>,
    prog: Vec<u8>,
}

fn save_ir(path: &str, ir: &IrSeq) {
    let bytes = bincode::serialize(ir).expect("failed to serialize IR");
    std::fs::write(path, bytes).expect("failed to write IR file");
}

fn load_ir(path: &str) -> IrSeq {
    let bytes = std::fs::read(path).expect("failed to read IR file");
    bincode::deserialize(&bytes).expect("failed to deserialize IR")
}

fn load_ir_auto(path: &str) -> IrSeq {
    if path.ends_with(".json") {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", path, e));
        let meta: InputMeta =
            serde_json::from_str(&content).unwrap_or_else(|e| panic!("failed to parse {}: {}", path, e));
        sbpf2ir(&meta.asm, meta.memory, &meta.version)
    } else {
        load_ir(path)
    }
}

fn print_ir(ir: &IrSeq) {
    for (region_id, nodes) in &ir.regions {
        if nodes.is_empty() {
            continue;
        }
        println!("[{}]", region_id);
        for node in nodes {
            println!("  {}", node);
        }
    }
}

// --- Demo mode ---

fn demo(label: &str, code: &str, memory: Vec<u8>, version: &str) {
    println!("=== {} ===", label);
    let ir_seq = sbpf2ir(code, memory, version);
    print_ir(&ir_seq);
    println!();
}

fn run_demos() {
    demo(
        "test_ja: unconditional forward jump",
        "add64 r10, 0
         mov r0, 1
         ja +1
         mov r0, 2
         exit",
        vec![],
        "V0",
    );

    demo(
        "test_lmul_loop: loop with jeq exit + jne back-edge",
        "add64 r10, 0
         mov r0, 0x7
         add r1, 0xa
         lsh r1, 0x20
         rsh r1, 0x20
         jeq r1, 0x0, +4
         mov r0, 0x7
         mul r0, 0x7
         add r1, -1
         jne r1, 0x0, -3
         exit",
        vec![],
        "V0",
    );

    demo(
        "test_prime: nested loops for primality check",
        "add64 r10, 0
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
         exit",
        vec![],
        "V0",
    );

    demo(
        "test_subnet: cascading jne/jeq decision tree",
        "add64 r10, 0
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
        vec![],
        "V0",
    );

    demo(
        "recursive call: call function_foo with jeq base case",
        "add64 r10, 0
         ldxb r1, [r1]
         add64 r1, -2
         call function_foo
         exit
         function_foo:
         add64 r10, 0
         jeq r1, 0, +2
         add64 r1, -1
         call function_foo
         exit",
        vec![5],
        "V0",
    );
}

// --- CLI ---

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  sbpf-ir                                    Demo mode (built-in examples)");
    eprintln!("  sbpf-ir <input.json> [-o <out.ir>]         Load JSON, translate to IR");
    eprintln!("  sbpf-ir --load <file.ir>                   Load saved IR, print regions");
    eprintln!(
        "  sbpf-ir --mutate <f1> <f2> ... [-o out.ir]  Mutate k IRs (.ir or .json)"
    );
    eprintln!("  sbpf-ir --seed <N>                         Set RNG seed (with --mutate)");
    eprintln!("  sbpf-ir --exec <prog.json>                 Run interpreter/JIT diff test");
    eprintln!("  sbpf-ir --triage <file.ir>                  Triage: asm, disasm, verify, exec");
    eprintln!("  sbpf-ir --gen-pqr [output_dir]              Generate PQR IR corpus (default: input_corpus/)");
    eprintln!("  sbpf-ir --gen-corpus [output_dir]            Generate full IR corpus (default: input_corpus/)");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        run_demos();
        return;
    }

    let mut mutate_mode = false;
    let mut load_mode = false;
    let mut exec_mode = false;
    let mut triage_mode = false;
    let mut gen_pqr_mode = false;
    let mut gen_corpus_mode = false;
    let mut input_files: Vec<String> = Vec::new();
    let mut output_path: Option<String> = None;
    let mut seed: Option<u64> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mutate" => {
                mutate_mode = true;
            }
            "--exec" => {
                exec_mode = true;
            }
            "--triage" => {
                triage_mode = true;
            }
            "--gen-pqr" => {
                gen_pqr_mode = true;
            }
            "--gen-corpus" => {
                gen_corpus_mode = true;
            }
            "--load" => {
                load_mode = true;
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --load requires a path");
                    print_usage();
                    std::process::exit(1);
                }
                input_files.push(args[i].clone());
            }
            "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: -o requires a path");
                    print_usage();
                    std::process::exit(1);
                }
                output_path = Some(args[i].clone());
            }
            "--seed" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --seed requires a number");
                    print_usage();
                    std::process::exit(1);
                }
                seed = Some(args[i].parse().expect("--seed requires a number"));
            }
            "--help" | "-h" => {
                print_usage();
                return;
            }
            other => {
                input_files.push(other.to_string());
            }
        }
        i += 1;
    }

    // --load mode
    if load_mode {
        let ir = load_ir(&input_files[0]);
        println!("version: {}", ir.version);
        println!("memory: {:?}", ir.memory);
        print_ir(&ir);
        return;
    }

    // --gen-pqr mode
    if gen_pqr_mode {
        let dir = input_files.first().map(|s| s.as_str()).unwrap_or("input_corpus");
        gen_pqr::generate(dir);
        return;
    }

    // --gen-corpus mode
    if gen_corpus_mode {
        let dir = input_files.first().map(|s| s.as_str()).unwrap_or("input_corpus");
        let pqr_count = gen_pqr::generate(dir);
        let total = gen_corpus::generate(dir, pqr_count);
        println!("Total: {} IR corpus files in {}", total, dir);
        return;
    }

    // --triage mode
    if triage_mode {
        if input_files.is_empty() {
            eprintln!("error: --triage requires an input file (.ir or .json)");
            print_usage();
            std::process::exit(1);
        }
        let ir = load_ir_auto(&input_files[0]);
        let sbpf_version = parse_sbpf_version(&ir.version)
            .unwrap_or_else(|| panic!("unknown SBPF version: {}", ir.version));
        triage_ir(&ir, sbpf_version);
        return;
    }

    // --exec mode
    if exec_mode {
        if input_files.is_empty() {
            eprintln!("error: --exec requires a JSON input file");
            print_usage();
            std::process::exit(1);
        }
        let content = std::fs::read_to_string(&input_files[0]).expect("failed to read exec JSON");
        let meta: ExecMeta = serde_json::from_str(&content).expect("failed to parse exec JSON");
        let sbpf_version = parse_sbpf_version(&meta.version)
            .unwrap_or_else(|| panic!("unknown SBPF version: {}", meta.version));
        let config = Config::default();
        let result = run_diff(&meta.prog, &meta.memory, config, sbpf_version);
        println!("{:?}", result);
        return;
    }

    // --mutate mode
    if mutate_mode {
        if input_files.is_empty() {
            eprintln!("error: --mutate requires at least 1 input file");
            print_usage();
            std::process::exit(1);
        }
        let irs: Vec<IrSeq> = input_files.iter().map(|p| load_ir_auto(p)).collect();
        let mut rng: StdRng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };
        let result = mutate(&irs, &mut rng);
        if let Some(out) = output_path {
            save_ir(&out, &result);
            println!("mutated IR saved to {}", out);
        } else {
            println!("version: {}", result.version);
            println!("memory: {:?}", result.memory);
            print_ir(&result);
        }
        return;
    }

    // JSON input mode
    if let Some(path) = input_files.first() {
        let content = std::fs::read_to_string(path).expect("failed to read input JSON");
        let meta: InputMeta = serde_json::from_str(&content).expect("failed to parse input JSON");
        let ir = sbpf2ir(&meta.asm, meta.memory, &meta.version);
        if let Some(out) = output_path {
            save_ir(&out, &ir);
            println!("IR saved to {}", out);
        } else {
            println!("version: {}", ir.version);
            println!("memory: {:?}", ir.memory);
            print_ir(&ir);
        }
        return;
    }

    print_usage();
    std::process::exit(1);
}
