#![warn(missing_docs)]

//! IR-based sBPF stress generation and lowering helpers.

pub mod generator;
pub mod ir;
pub mod lowering;
pub mod validate;
