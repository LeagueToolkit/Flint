// Project management module exports
#[allow(clippy::module_inception)]
pub mod project;

pub use project::{create_project, open_project, save_project, Project};
