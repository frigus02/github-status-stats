mod parser;

use parser::{parse, TransformInstruction, TransformKind};
use std::env;

pub struct Transform {
    instructions: Vec<TransformInstruction>,
}

impl Transform {
    pub fn transform(&self, input: String) -> String {
        self.instructions
            .iter()
            .fold(input, |input, instruction| match instruction.kind {
                TransformKind::Substitute => {
                    input.replace(&instruction.args[0], &instruction.args[1])
                }
            })
    }
}

fn create_transform(transform: &str) -> Result<Transform, String> {
    let instructions = parse(transform)?;
    Ok(Transform { instructions })
}

pub fn create_transform_from_env(key: &str) -> Result<Transform, String> {
    match env::var(key) {
        Ok(transform) => create_transform(&transform),
        Err(_) => Ok(Transform {
            instructions: Vec::new(),
        }),
    }
}
