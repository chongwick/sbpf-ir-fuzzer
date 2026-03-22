use crate::ir::{sbpf2ir, IrSeq};
use crate::semantic_aware::{make_program, FuzzProgram};
use arbitrary::{Arbitrary, Unstructured};
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use solana_sbpf::insn_builder::IntoBytes;
use solana_sbpf::program::{BuiltinProgram, FunctionRegistry, SBPFVersion};
use solana_sbpf::static_analysis::Analysis;
use solana_sbpf::verifier::{RequisiteVerifier, Verifier};
use solana_sbpf::vm::Config;
use std::path::Path;
use std::sync::Arc;
use test_utils::TestContextObject;

fn save_ir(path: &str, ir: &IrSeq) {
    let bytes = bincode::serialize(ir).expect("failed to serialize IR");
    std::fs::write(path, bytes).expect("failed to write IR file");
}

fn version_str(v: SBPFVersion) -> &'static str {
    match v {
        SBPFVersion::V0 => "V0",
        SBPFVersion::V1 => "V1",
        SBPFVersion::V2 => "V2",
        SBPFVersion::V3 => "V3",
        SBPFVersion::V4 => "V4",
        _ => "V0",
    }
}

/// Generate smart IR seeds using semantic-aware instruction generation.
/// Removes old `smart_*.ir` files from the output directory first.
/// Returns the number of seeds successfully generated.
pub fn generate(output_dir: &str, count: usize, seed: u64) -> usize {
    let path = Path::new(output_dir);
    std::fs::create_dir_all(path).unwrap();

    // Remove old smart seeds
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name
                .to_str()
                .map_or(false, |n| n.starts_with("smart_") && n.ends_with(".ir"))
            {
                std::fs::remove_file(entry.path()).ok();
            }
        }
    }

    let mut rng = StdRng::seed_from_u64(seed);
    let mut generated = 0;

    let versions = [
        SBPFVersion::V0,
        SBPFVersion::V1,
        SBPFVersion::V2,
        SBPFVersion::V3,
        SBPFVersion::V4,
    ];

    // Attempt up to 10x count to hit target (some programs may fail to build/verify)
    for i in 0..count * 10 {
        if generated >= count {
            break;
        }

        // Generate random bytes for Arbitrary
        let mut raw = vec![0u8; 4096];
        rng.fill_bytes(&mut raw);
        let mut u = Unstructured::new(&raw);
        let prog: FuzzProgram = match FuzzProgram::arbitrary(&mut u) {
            Ok(p) if !p.is_empty() => p,
            _ => continue,
        };

        let version = versions[i % versions.len()];
        let code = make_program(&prog, version);
        let bytecode = code.into_bytes();

        let config = Config {
            enabled_sbpf_versions: version..=version,
            ..Default::default()
        };
        let loader = Arc::new(BuiltinProgram::new_loader(config.clone()));

        // Build executable from bytecode
        let executable =
            match solana_sbpf::elf::Executable::<TestContextObject>::from_text_bytes(
                bytecode,
                loader,
                version,
                FunctionRegistry::default(),
            ) {
                Ok(e) => e,
                Err(_) => continue,
            };

        // Verify
        let text_bytes = executable.get_text_bytes().1;
        if RequisiteVerifier::verify(text_bytes, &config, version).is_err() {
            continue;
        }

        // Disassemble to assembly text
        let analysis = match Analysis::from_executable(&executable) {
            Ok(a) => a,
            Err(_) => continue,
        };
        let mut asm_buf = Vec::new();
        if analysis.disassemble(&mut asm_buf).is_err() {
            continue;
        }
        let asm_text = match String::from_utf8(asm_buf) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Convert disassembly to IR
        let v_str = version_str(version);
        let ir = sbpf2ir(&asm_text, vec![], v_str);

        let name = format!("smart_{:04}_{}.ir", generated, v_str.to_lowercase());
        save_ir(path.join(&name).to_str().unwrap(), &ir);
        generated += 1;
    }

    println!(
        "Generated {} smart IR seeds in {}",
        generated, output_dir
    );
    generated
}
