use rand::Rng;

use crate::ir::{IrNode, IrSeq};

// --- Opcode category tables ---

const ALU64_OPS: &[&str] = &[
    "add64", "sub64", "mul64", "div64", "or64", "and64", "lsh64", "rsh64", "xor64", "mov64",
    "arsh64", "mod64",
];
const ALU32_OPS: &[&str] = &[
    "add32", "sub32", "mul32", "div32", "or32", "and32", "lsh32", "rsh32", "xor32", "mov32",
    "arsh32", "mod32",
];
const LOAD_OPS: &[&str] = &["ldxb", "ldxh", "ldxw", "ldxdw"];
const STORE_IMM_OPS: &[&str] = &["stb", "sth", "stw", "stdw"];
const STORE_REG_OPS: &[&str] = &["stxb", "stxh", "stxw", "stxdw"];
const JUMP_OPS: &[&str] = &[
    "jeq", "jne", "jgt", "jge", "jlt", "jle", "jsgt", "jsge", "jslt", "jsle", "jset",
];
const JUMP32_OPS: &[&str] = &[
    "jeq32", "jne32", "jgt32", "jge32", "jlt32", "jle32", "jsgt32", "jsge32", "jslt32", "jsle32",
    "jset32",
];
const PQR32_OPS: &[&str] = &["udiv32", "urem32", "lmul32", "sdiv32", "srem32"];
const PQR64_OPS: &[&str] = &[
    "udiv64", "urem64", "lmul64", "sdiv64", "srem64", "uhmul64", "shmul64",
];
const OP_CATEGORIES: &[&[&str]] = &[
    ALU64_OPS,
    ALU32_OPS,
    LOAD_OPS,
    STORE_IMM_OPS,
    STORE_REG_OPS,
    JUMP_OPS,
    JUMP32_OPS,
    PQR32_OPS,
    PQR64_OPS,
];
const EDGE_IMMEDIATES: &[&str] = &[
    "0",
    "1",
    "-1",
    "0x7fffffff",
    "-2147483648",
    "0xffffffff",
    "0xff",
    "0xffff",
    "0x100",
    "0x10000",
];

// --- Helpers ---

fn find_category(op: &str) -> Option<&'static [&'static str]> {
    for cat in OP_CATEGORIES {
        if cat.contains(&op) {
            return Some(cat);
        }
    }
    None
}

fn is_register(s: &str) -> bool {
    s.starts_with('r') && s.len() <= 3 && s[1..].chars().all(|c| c.is_ascii_digit())
}

fn is_immediate(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Hex
    if s.starts_with("0x") || s.starts_with("-0x") {
        return true;
    }
    // Decimal (possibly negative)
    let s = s.strip_prefix('-').unwrap_or(s);
    s.chars().all(|c| c.is_ascii_digit())
}

fn is_offset(s: &str) -> bool {
    s.starts_with('+') || s.starts_with('-')
}

fn is_memory_operand(s: &str) -> bool {
    s.starts_with('[') || s.ends_with(']')
}

// --- Point mutation operators ---

/// Pick a random instruction and swap its op with another in the same category.
fn op_substitute(ir: &mut IrSeq, rng: &mut impl Rng) {
    // Collect all (region_idx, node_idx) pairs whose op is in a known category
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    for (ri, (_, nodes)) in ir.regions.iter().enumerate() {
        for (ni, node) in nodes.iter().enumerate() {
            if find_category(&node.op).is_some() {
                candidates.push((ri, ni));
            }
        }
    }
    if candidates.is_empty() {
        return;
    }
    let (ri, ni) = candidates[rng.gen_range(0..candidates.len())];
    let cat = find_category(&ir.regions[ri].1[ni].op).unwrap();
    if cat.len() <= 1 {
        return;
    }
    let new_op = loop {
        let candidate = cat[rng.gen_range(0..cat.len())];
        if candidate != ir.regions[ri].1[ni].op {
            break candidate;
        }
    };
    ir.regions[ri].1[ni].op = new_op.to_string();
}

/// Pick a random operand and mutate it: registers -> random r0-r9,
/// immediates -> 50% edge value / 50% small random, offsets -> small perturbation.
fn operand_mutate(ir: &mut IrSeq, rng: &mut impl Rng) {
    // Collect all (region, node, operand_idx) triples with mutable operands
    let mut candidates: Vec<(usize, usize, usize)> = Vec::new();
    for (ri, (_, nodes)) in ir.regions.iter().enumerate() {
        for (ni, node) in nodes.iter().enumerate() {
            for (oi, operand) in node.operands.iter().enumerate() {
                if is_memory_operand(operand) {
                    continue;
                }
                if is_register(operand) || is_immediate(operand) || is_offset(operand) {
                    candidates.push((ri, ni, oi));
                }
            }
        }
    }
    if candidates.is_empty() {
        return;
    }
    let (ri, ni, oi) = candidates[rng.gen_range(0..candidates.len())];
    let operand = &ir.regions[ri].1[ni].operands[oi];
    let new_val = if is_register(operand) {
        format!("r{}", rng.gen_range(0..10u8))
    } else if is_offset(operand) {
        let delta: i16 = rng.gen_range(-5..=5);
        let cur: i16 = operand.parse().unwrap_or(0);
        let new = cur.saturating_add(delta);
        if new >= 0 {
            format!("+{}", new)
        } else {
            format!("{}", new)
        }
    } else {
        // Immediate
        if rng.gen_bool(0.5) {
            EDGE_IMMEDIATES[rng.gen_range(0..EDGE_IMMEDIATES.len())].to_string()
        } else {
            format!("{}", rng.gen_range(-256..=256))
        }
    };
    ir.regions[ri].1[ni].operands[oi] = new_val;
}

/// Remove a random non-exit instruction from a region that has >= 2 instructions.
fn node_delete(ir: &mut IrSeq, rng: &mut impl Rng) {
    // Collect (region_idx, node_idx) of deletable nodes
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    for (ri, (_, nodes)) in ir.regions.iter().enumerate() {
        if nodes.len() < 2 {
            continue;
        }
        for (ni, node) in nodes.iter().enumerate() {
            if node.op != "exit" {
                candidates.push((ri, ni));
            }
        }
    }
    if candidates.is_empty() {
        return;
    }
    let (ri, ni) = candidates[rng.gen_range(0..candidates.len())];
    ir.regions[ri].1.remove(ni);
}

/// Swap two adjacent non-exit instructions in the same region.
fn node_swap(ir: &mut IrSeq, rng: &mut impl Rng) {
    // Collect (region_idx, node_idx) pairs where both idx and idx+1 are non-exit
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    for (ri, (_, nodes)) in ir.regions.iter().enumerate() {
        if nodes.len() < 2 {
            continue;
        }
        for ni in 0..nodes.len() - 1 {
            if nodes[ni].op != "exit" && nodes[ni + 1].op != "exit" {
                candidates.push((ri, ni));
            }
        }
    }
    if candidates.is_empty() {
        return;
    }
    let (ri, ni) = candidates[rng.gen_range(0..candidates.len())];
    ir.regions[ri].1.swap(ni, ni + 1);
}

// --- Splice infrastructure ---

enum Splice {
    /// Instructions to insert inline at a random position in the base.
    Inline(Vec<IrNode>),
    /// A new callable region plus a call instruction inserted at a random position.
    Region { name: String, nodes: Vec<IrNode> },
}

/// Extract a splice from a donor IR. Returns None if the donor has no usable instructions.
fn extract_splice(donor: &IrSeq, splice_id: &mut u64, rng: &mut impl Rng) -> Option<Splice> {
    let eligible: Vec<usize> = donor
        .regions
        .iter()
        .enumerate()
        .filter(|(_, (_, nodes))| nodes.iter().any(|n| n.op != "exit"))
        .map(|(i, _)| i)
        .collect();
    if eligible.is_empty() {
        return None;
    }

    match rng.gen_range(0u8..3) {
        // Single instruction
        0 => {
            let all: Vec<&IrNode> = donor
                .regions
                .iter()
                .flat_map(|(_, nodes)| nodes.iter())
                .filter(|n| n.op != "exit")
                .collect();
            if all.is_empty() {
                return None;
            }
            let idx = rng.gen_range(0..all.len());
            Some(Splice::Inline(vec![all[idx].clone()]))
        }
        // Contiguous instruction sequence from one region
        1 => {
            let r = eligible[rng.gen_range(0..eligible.len())];
            let non_exit: Vec<IrNode> = donor.regions[r]
                .1
                .iter()
                .filter(|n| n.op != "exit")
                .cloned()
                .collect();
            if non_exit.is_empty() {
                return None;
            }
            let start = rng.gen_range(0..non_exit.len());
            let len = rng.gen_range(1..=non_exit.len() - start);
            Some(Splice::Inline(non_exit[start..start + len].to_vec()))
        }
        // Entire region as a new callable function
        _ => {
            let r = eligible[rng.gen_range(0..eligible.len())];
            let mut nodes: Vec<IrNode> = donor.regions[r]
                .1
                .iter()
                .filter(|n| n.op != "exit")
                .cloned()
                .collect();
            if nodes.is_empty() {
                return None;
            }
            let name = format!("function_spliced_{}", splice_id);
            *splice_id += 1;
            // Spliced region needs exit so it can return from call
            nodes.push(IrNode {
                op: "exit".to_string(),
                operands: vec![],
            });
            Some(Splice::Region { name, nodes })
        }
    }
}

/// Collect all valid insertion points: (region_index, position 0..=len).
fn insertion_points(ir: &IrSeq) -> Vec<(usize, usize)> {
    let mut points = Vec::new();
    for (r, (_, nodes)) in ir.regions.iter().enumerate() {
        if nodes.is_empty() {
            continue;
        }
        for pos in 0..=nodes.len() {
            points.push((r, pos));
        }
    }
    points
}

/// Insert a splice into the base IR at a random position.
fn apply_splice(base: &mut IrSeq, splice: Splice, rng: &mut impl Rng) {
    let points = insertion_points(base);
    if points.is_empty() {
        return;
    }

    match splice {
        Splice::Inline(nodes) => {
            let (r, pos) = points[rng.gen_range(0..points.len())];
            for (i, node) in nodes.into_iter().enumerate() {
                base.regions[r].1.insert(pos + i, node);
            }
        }
        Splice::Region { name, nodes } => {
            base.regions.push((name.clone(), nodes));
            let (r, pos) = points[rng.gen_range(0..points.len())];
            base.regions[r].1.insert(
                pos,
                IrNode {
                    op: "call".to_string(),
                    operands: vec![name],
                },
            );
        }
    }
}

/// Ensure the last instruction of the last non-empty region is "exit".
fn ensure_exit_at_end(ir: &mut IrSeq) {
    if let Some((_, nodes)) = ir
        .regions
        .iter_mut()
        .rev()
        .find(|(_, nodes)| !nodes.is_empty())
    {
        if nodes.last().map_or(true, |n| n.op != "exit") {
            nodes.push(IrNode {
                op: "exit".to_string(),
                operands: vec![],
            });
        }
    } else {
        if ir.regions.is_empty() {
            ir.regions.push(("__region_0".to_string(), vec![]));
        }
        ir.regions[0].1.push(IrNode {
            op: "exit".to_string(),
            operands: vec![],
        });
    }
}

/// Mutate by picking a base IR and splicing from each compatible donor,
/// then applying 1-5 random point mutations.
pub fn mutate(irs: &[IrSeq], rng: &mut impl Rng) -> IrSeq {
    assert!(!irs.is_empty(), "need at least 1 IR");

    let base_idx = rng.gen_range(0..irs.len());
    let mut base = irs[base_idx].clone();
    let mut splice_id = 0u64;

    for (i, donor) in irs.iter().enumerate() {
        if i == base_idx {
            continue;
        }
        if donor.version != base.version {
            continue;
        }
        if let Some(splice) = extract_splice(donor, &mut splice_id, rng) {
            apply_splice(&mut base, splice, rng);
        }
    }

    // Apply 1-5 random point mutations
    let num_mutations = rng.gen_range(1..=5);
    for _ in 0..num_mutations {
        match rng.gen_range(0u8..4) {
            0 => op_substitute(&mut base, rng),
            1 => operand_mutate(&mut base, rng),
            2 => node_delete(&mut base, rng),
            _ => node_swap(&mut base, rng),
        }
    }

    ensure_exit_at_end(&mut base);
    base
}
