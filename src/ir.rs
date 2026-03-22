use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone)]
pub struct IrNode {
    pub op: String,
    pub operands: Vec<String>,
}

impl fmt::Display for IrNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.op)?;
        for (i, operand) in self.operands.iter().enumerate() {
            if i == 0 {
                write!(f, " {}", operand)?;
            } else {
                write!(f, ", {}", operand)?;
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IrSeq {
    pub version: String,
    pub memory: Vec<u8>,
    pub regions: Vec<(String, Vec<IrNode>)>,
    #[serde(skip)]
    generated_ids: Vec<String>,
    #[serde(skip)]
    current_region: usize,
    #[serde(skip)]
    next_id: u64,
}

impl IrSeq {
    pub fn new(version: &str, memory: Vec<u8>) -> Self {
        let mut seq = IrSeq {
            version: version.to_string(),
            memory,
            regions: Vec::new(),
            generated_ids: Vec::new(),
            current_region: 0,
            next_id: 0,
        };
        let id = seq.generate_id();
        seq.regions.push((id, Vec::new()));
        seq
    }

    fn generate_id(&mut self) -> String {
        let id = format!("__region_{}", self.next_id);
        self.next_id += 1;
        self.generated_ids.push(id.clone());
        id
    }

    fn add_node(&mut self, node: IrNode) {
        self.regions[self.current_region].1.push(node);
    }

    fn change_region(&mut self, name: &str) {
        if name.is_empty() {
            let id = self.generate_id();
            self.regions.push((id, Vec::new()));
            self.current_region = self.regions.len() - 1;
        } else if let Some(idx) = self.regions.iter().position(|(id, _)| id == name) {
            self.current_region = idx;
        } else {
            self.regions.push((name.to_string(), Vec::new()));
            self.current_region = self.regions.len() - 1;
        }
    }
}

impl fmt::Display for IrSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (region_id, nodes) in &self.regions {
            if nodes.is_empty() {
                continue;
            }
            if !self.generated_ids.contains(region_id) {
                writeln!(f, "{}:", region_id)?;
            }
            for node in nodes {
                writeln!(f, "{}", node)?;
            }
        }
        Ok(())
    }
}

fn is_conditional_branch(op: &str) -> bool {
    matches!(
        op,
        "jeq" | "jgt" | "jge" | "jlt" | "jle" | "jset" | "jne"
            | "jsgt" | "jsge" | "jslt" | "jsle"
            | "jeq32" | "jgt32" | "jge32" | "jlt32" | "jle32" | "jset32" | "jne32"
            | "jsgt32" | "jsge32" | "jslt32" | "jsle32"
    )
}

fn is_unconditional_jump(op: &str) -> bool {
    op == "ja"
}

pub fn sbpf2ir(code: &str, memory: Vec<u8>, version: &str) -> IrSeq {
    let mut ir_seq = IrSeq::new(version, memory);
    let mut need_new_region = false;

    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.ends_with(':') {
            let label = &trimmed[..trimmed.len() - 1];
            ir_seq.change_region(label);
            need_new_region = false;
            continue;
        }

        if need_new_region {
            ir_seq.change_region("");
            need_new_region = false;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let op = parts[0];
        let operands: Vec<String> = parts[1..]
            .iter()
            .map(|s| s.trim_end_matches(',').to_string())
            .collect();

        ir_seq.add_node(IrNode {
            op: op.to_string(),
            operands: operands.clone(),
        });

        if is_conditional_branch(op) || is_unconditional_jump(op) || op == "exit" {
            need_new_region = true;
        }
    }

    ir_seq
}
