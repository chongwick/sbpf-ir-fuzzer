use std::sync::Arc;

use solana_sbpf::{
    assembler::assemble,
    ebpf,
    elf::Executable,
    memory_region::MemoryRegion,
    program::{BuiltinProgram, FunctionRegistry, SBPFVersion},
    static_analysis::Analysis,
    verifier::{RequisiteVerifier, Verifier},
    vm::{Config, ExecutionMode},
};
use test_utils::{create_vm, TestContextObject};

use crate::ir::IrSeq;

pub fn parse_sbpf_version(s: &str) -> Option<SBPFVersion> {
    match s {
        "V0" => Some(SBPFVersion::V0),
        "V1" => Some(SBPFVersion::V1),
        "V2" => Some(SBPFVersion::V2),
        "V3" => Some(SBPFVersion::V3),
        "V4" => Some(SBPFVersion::V4),
        _ => None,
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum DiffResult {
    VerifyFailed,
    AssemblyFailed(String),
    Success {
        interp: Result<u64, String>,
        #[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
        jit: Option<Result<u64, String>>,
    },
}

/// Run interpreter (and JIT on supported platforms) on an already-built Executable,
/// panicking on any differential mismatch.
fn run_executable(
    #[allow(unused_mut)] mut executable: Executable<TestContextObject>,
    mem: &[u8],
) -> DiffResult {
    // --- Interpreter ---
    let mut interp_mem = mem.to_vec();
    let mut interp_context_object = TestContextObject::new(1 << 16);
    let interp_mem_region = MemoryRegion::new_writable(&mut interp_mem, ebpf::MM_INPUT_START);
    create_vm!(
        interp_vm,
        &executable,
        &mut interp_context_object,
        interp_stack,
        interp_heap,
        vec![interp_mem_region],
        None
    );
    #[allow(unused)]
    let (_interp_ins_count, interp_res) =
        interp_vm.execute_program(&executable, &mut ExecutionMode::Interpreted);
    #[allow(unused)]
    let interp_final_pc = interp_vm.registers[11];

    let interp_result = match &interp_res {
        solana_sbpf::error::StableResult::Ok(v) => Ok(*v),
        solana_sbpf::error::StableResult::Err(e) => Err(format!("{:?}", e)),
    };

    // --- JIT ---
    #[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
    {
        if executable.jit_compile().is_ok() {
            let mut jit_mem = mem.to_vec();
            let mut jit_context_object = TestContextObject::new(1 << 16);
            let jit_mem_region = MemoryRegion::new_writable(&mut jit_mem, ebpf::MM_INPUT_START);
            create_vm!(
                jit_vm,
                &executable,
                &mut jit_context_object,
                jit_stack,
                jit_heap,
                vec![jit_mem_region],
                None
            );
            let (_jit_ins_count, jit_res) =
                jit_vm.execute_program(&executable, &mut ExecutionMode::Jit);
            let jit_final_pc = jit_vm.registers[11];

            if format!("{:?}", interp_res) != format!("{:?}", jit_res) {
                panic!("Expected {:?}, but got {:?}", interp_res, jit_res);
            }
            if interp_res.is_ok() {
                if interp_context_object.remaining != jit_context_object.remaining {
                    panic!(
                        "Expected {} insts remaining, but got {}",
                        interp_context_object.remaining, jit_context_object.remaining
                    );
                }
                if interp_mem != jit_mem {
                    panic!(
                        "Expected different memory. From interpreter: {:?}\nFrom JIT: {:?}",
                        interp_mem, jit_mem
                    );
                }
                if interp_stack != jit_stack {
                    panic!(
                        "Expected different stack. From interpreter: {:?}\nFrom JIT: {:?}",
                        interp_stack, jit_stack
                    );
                }
                if interp_heap != jit_heap {
                    panic!(
                        "Expected different heap. From interpreter: {:?}\nFrom JIT: {:?}",
                        interp_heap, jit_heap
                    );
                }
            }
            if interp_final_pc != jit_final_pc {
                panic!(
                    "Expected final PC {}, but got {}",
                    interp_final_pc, jit_final_pc
                );
            }

            let jit_result = match jit_res {
                solana_sbpf::error::StableResult::Ok(v) => Ok(v),
                solana_sbpf::error::StableResult::Err(e) => Err(format!("{:?}", e)),
            };
            return DiffResult::Success {
                interp: interp_result,
                jit: Some(jit_result),
            };
        } else {
            panic!("JIT compilation failed for program that passed verification");
        }
    }

    #[cfg(not(all(not(target_os = "windows"), target_arch = "x86_64")))]
    DiffResult::Success {
        interp: interp_result,
    }
}

/// Verify + execute raw bytecode through the differential oracle.
pub fn run_diff(prog: &[u8], mem: &[u8], config: Config, sbpf_version: SBPFVersion) -> DiffResult {
    let function_registry = FunctionRegistry::default();

    if RequisiteVerifier::verify(prog, &config, sbpf_version).is_err() {
        return DiffResult::VerifyFailed;
    }

    let executable = Executable::<TestContextObject>::from_text_bytes(
        prog,
        Arc::new(BuiltinProgram::new_loader(config)),
        sbpf_version,
        function_registry,
    )
    .unwrap();

    run_executable(executable, mem)
}

/// Assemble an IrSeq into bytecode and execute through the differential oracle.
pub fn run_diff_ir(ir: &IrSeq, mut config: Config, sbpf_version: SBPFVersion) -> DiffResult {
    config.enabled_sbpf_versions = sbpf_version..=sbpf_version;
    let asm = format!("{}", ir);
    let loader = Arc::new(BuiltinProgram::new_loader(config.clone()));
    let executable = match assemble::<TestContextObject>(&asm, loader) {
        Ok(exe) => exe,
        Err(e) => return DiffResult::AssemblyFailed(e),
    };
    let text_bytes = executable.get_text_bytes().1;
    if RequisiteVerifier::verify(text_bytes, &config, sbpf_version).is_err() {
        return DiffResult::VerifyFailed;
    }
    run_executable(executable, &ir.memory)
}

/// Triage a crash: print assembly, disassembly, verify status, and execution result.
/// When `skip_verify` is true, execution proceeds even if verification fails.
pub fn triage_ir(ir: &IrSeq, sbpf_version: SBPFVersion, skip_verify: bool) {
    // 1. Assembly text
    let asm = format!("{}", ir);
    println!("=== Assembly text ===");
    for (i, line) in asm.lines().enumerate() {
        println!("  {:3}: {}", i, line);
    }
    println!();

    // 2. Assemble
    let config = Config {
        enabled_sbpf_versions: sbpf_version..=sbpf_version,
        ..Config::default()
    };
    let loader = Arc::new(BuiltinProgram::new_loader(config.clone()));
    let executable = match assemble::<TestContextObject>(&asm, loader) {
        Ok(exe) => exe,
        Err(e) => {
            println!("=== Assembly FAILED ===");
            println!("  {}", e);
            return;
        }
    };

    // 3. Disassembly
    println!("=== Disassembly ===");
    match Analysis::from_executable(&executable) {
        Ok(analysis) => {
            let mut buf = Vec::new();
            let _ = analysis.disassemble(&mut buf);
            print!("{}", String::from_utf8_lossy(&buf));
        }
        Err(e) => println!("  analysis failed: {:?}", e),
    }
    println!();

    // 4. Verify
    let text_bytes = executable.get_text_bytes().1;
    let verify_result = RequisiteVerifier::verify(text_bytes, &config, sbpf_version);
    println!("=== Verification ===");
    match &verify_result {
        Ok(()) => println!("  PASSED"),
        Err(e) => println!("  FAILED: {:?}", e),
    }
    println!();

    // 5. Execute (only if verified, unless --no-verify)
    println!("=== Execution ===");
    if verify_result.is_err() && !skip_verify {
        println!("  skipped (verification failed)");
        return;
    }
    if verify_result.is_err() {
        println!("  WARNING: running unverified program (--no-verify)");
    }
    let result = run_executable(executable, &ir.memory);
    println!("  {:?}", result);
}
