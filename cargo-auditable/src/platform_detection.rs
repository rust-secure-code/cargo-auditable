//! Utilities to reliably and consistently detect various platforms

use crate::target_info::RustcTargetInfo;

pub fn is_wasm(target_info: &RustcTargetInfo) -> bool {
    key_equals(target_info, "target_family", "wasm")
}

pub fn is_msvc(target_info: &RustcTargetInfo) -> bool {
    key_equals(target_info, "target_env", "msvc")
}

pub fn is_apple(target_info: &RustcTargetInfo) -> bool {
    key_equals(target_info, "target_vendor", "apple")
}

pub fn is_windows(target_info: &RustcTargetInfo) -> bool {
    key_equals(target_info, "target_os", "windows")
}

pub fn is_32bit(target_info: &RustcTargetInfo) -> bool {
    key_equals(target_info, "target_pointer_width", "32")
}

fn key_equals(target_info: &RustcTargetInfo, key: &str, value: &str) -> bool {
    target_info.get(key).map(|s| s.as_str()) == Some(value)
}
