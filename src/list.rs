use std::fmt::Display;

use crate::config::{Config, LoadedConfig};

pub enum ListError {}

impl Display for ListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub fn list(config: &LoadedConfig) -> Result<(), ListError> {
    
    
    Ok(())
}