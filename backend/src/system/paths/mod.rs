// Path mapping and resolution module
// Handles cross-platform path mapping and template resolution

pub mod mapper;
pub mod platform;
pub mod service;

pub use mapper::PathMapper;
pub use service::{PathService, get_path_service};
