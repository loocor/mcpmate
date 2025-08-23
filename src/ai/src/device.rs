//! Device management module
//!
//! Manages GPU/CPU device initialization and prioritizes Metal acceleration

use crate::debug_println;
use anyhow::Result;
use candle_core::Device;

/// Device manager
pub struct DeviceManager;

impl DeviceManager {
    /// Create optimal device
    ///
    /// Uses Peeches strategy:
    /// - macOS: Force Metal, fail with error if not supported
    /// - Other platforms: Use CPU
    pub fn create_optimal_device() -> Result<Device> {
        let device = if cfg!(target_os = "macos") {
            Device::new_metal(0).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to initialize Metal device: {}. Metal support is required on macOS.",
                    e
                )
            })?
        } else {
            Device::Cpu
        };

        debug_println!("🚀 Using device: {:?}", device);
        Ok(device)
    }

    /// Check device capabilities
    pub fn check_device_capabilities(device: &Device) -> DeviceInfo {
        match device {
            Device::Cpu => DeviceInfo {
                device_type: DeviceType::Cpu,
                memory_gb: Self::get_system_memory_gb(),
                compute_capability: "CPU".to_string(),
            },
            Device::Metal(_) => DeviceInfo {
                device_type: DeviceType::Metal,
                memory_gb: Self::get_metal_memory_gb(),
                compute_capability: "Metal".to_string(),
            },
            Device::Cuda(_) => DeviceInfo {
                device_type: DeviceType::Cuda,
                memory_gb: Self::get_cuda_memory_gb(),
                compute_capability: "CUDA".to_string(),
            },
        }
    }

    fn get_system_memory_gb() -> f32 {
        // Simplified implementation, actual system call can be used to get
        16.0
    }

    fn get_metal_memory_gb() -> f32 {
        // Simplified implementation, actual Metal API can be used to get
        16.0
    }

    fn get_cuda_memory_gb() -> f32 {
        // Simplified implementation, actual CUDA API can be used to get
        8.0
    }
}

/// Device type
#[derive(Debug, Clone)]
pub enum DeviceType {
    Cpu,
    Metal,
    Cuda,
}

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_type: DeviceType,
    pub memory_gb: f32,
    pub compute_capability: String,
}

impl DeviceInfo {
    /// Whether supports high-performance inference
    pub fn supports_fast_inference(&self) -> bool {
        matches!(self.device_type, DeviceType::Metal | DeviceType::Cuda)
    }

    /// Get recommended batch size
    pub fn recommended_batch_size(&self) -> usize {
        match self.device_type {
            DeviceType::Cpu => 1,
            DeviceType::Metal => 4,
            DeviceType::Cuda => 8,
        }
    }
}
