#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = auditable_extract::raw_auditable_data_wasm_for_fuzz(data);
});
