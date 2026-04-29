mod engine;

pub use engine::ExecutionEngine;
use std::collections::HashMap;
use rhai::AST;
use crate::managers::driver::ActiveDriver;

pub struct CompiledDriver {
    pub driver: ActiveDriver,
    pub gateway_ast: AST,
    pub device_asts: HashMap<String, AST>,
}
