use rand::Rng;

use crate::ir::{IrNode, IrSeq};

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

/// Mutate by picking a base IR and splicing from each compatible donor.
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

    ensure_exit_at_end(&mut base);
    base
}
