//! Wrapper around object_file.rs to keep it as intact as possible,
//! because it is lifted from rustc

use crate::{object_file, platform_detection::is_wasm, target_info::RustcTargetInfo};

/// Creates a binary file (ELF/Mach-O/PE/WASM) with the specified contents in a given section
/// which can be passed to the linker to include the section into the final executable.
///
/// Returns `None` if the architecture is not supported.
pub fn create_binary_file(
    target_info: &RustcTargetInfo,
    target_triple: &str,
    contents: &[u8],
    symbol_name: &str,
) -> Option<Vec<u8>> {
    if is_wasm(target_info) {
        Some(create_wasm_file(target_info, contents))
    } else {
        object_file::create_metadata_file(target_info, target_triple, contents, symbol_name)
    }
}

pub fn create_wasm_file(
    // formerly `create_compressed_metadata_file` in the rustc codebase
    target_info: &RustcTargetInfo,
    contents: &[u8],
) -> Vec<u8> {
    assert!(is_wasm(target_info));

    // Start with the minimum valid WASM file
    let mut result: Vec<u8> = vec![0, b'a', b's', b'm', 1, 0, 0, 0];

    // Add the `linking` section with version 2 that rust-lld expects.
    // This is required to mark the WASM file as relocatable,
    // otherwise the linker will reject it as a non-linkable file.
    // https://github.com/WebAssembly/tool-conventions/blob/master/Linking.md
    wasm_gen::write_custom_section(&mut result, "linking", &[2]);

    wasm_gen::write_custom_section(&mut result, ".dep-v0", contents);
    result
}
