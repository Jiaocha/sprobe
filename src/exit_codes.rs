//! 稳定退出码定义（文档 8.9 节）

#![allow(dead_code)]

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_RUNTIME_ERROR: i32 = 1;
pub const EXIT_CONFIG_ERROR: i32 = 2;
pub const EXIT_ALREADY_RUNNING: i32 = 3;
pub const EXIT_PREREQUISITE_FAILED: i32 = 4;
pub const EXIT_UNSUPPORTED_PLATFORM: i32 = 5;
