pub mod host;
pub mod linux;
pub mod network;
pub mod uptime;

#[cfg(windows)]
pub mod windows;

pub use host::collect_host_meta;
pub use uptime::UptimeCollector;

#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "linux")]
pub use linux::*;
