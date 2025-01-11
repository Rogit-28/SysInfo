pub mod cpu;
pub mod memory;
pub mod gpu;
pub mod storage;
pub use cpu::CpuCollector;
pub use memory::MemoryCollector;
pub use gpu::GpuCollector;
pub use storage::StorageCollector;
