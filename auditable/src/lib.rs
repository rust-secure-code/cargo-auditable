#[used]
#[link_section = ".rust-audit-dep-list"]
static AUDITABLE_VERSION_INFO: [u8; include_bytes!(concat!(
    env!("OUT_DIR"),
    "/Cargo.lock.annotated"
))
.len()] = *include_bytes!(concat!(env!("OUT_DIR"), "/Cargo.lock.annotated"));

/// Returns the version info embedded into the executable at compile time.
#[inline]
pub fn version_info() -> &'static str {
    let _ = VERIFIED_UTF8; // suppress warnings about VERIFIED_UTF8 being unused
    unsafe { std::str::from_utf8_unchecked(&AUDITABLE_VERSION_INFO) } // see below
}

// ==== All of the below code is just validation for that `unsafe` ==== //

// Sadly we cannot yet use `TryFrom` in `const fn` to cast &[u8] to &[u8; KNOWN_SIZE],
// so we manually verify that what we've just read was valid UTF-8
// by matching it against the same data read as a string, byte-by-byte.
// The success of reading the string on its own would not be sufficient because
// it would not protect us from time-of-check/time-of-use attacks.
// Admittedly whoever can mess with build.rs output dir can likely mess with the
// output binary as well, but better safe than sorry.
static VERIFIED_UTF8: () = {
    let data_to_verify: &[u8] = &AUDITABLE_VERSION_INFO; // cast to unsized slice
    let data_as_valid_string: &str =
        include_str!(concat!(env!("OUT_DIR"), "/Cargo.lock.annotated"));
    if !slices_are_equal(data_to_verify, data_as_valid_string.as_bytes()) {
        fail_build_on_invalid_utf8_in_Cargo_toml();
    }
};

const fn slices_are_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    };
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

// FIXME: name is misleading, this is actually toc/tou protection
#[allow(unconditional_panic)]
const fn dependency_file_generated_by_build_rs_was_modified_while_I_was_reading_it() {
    [()][1337]; // because panicking in `const fn` is unstable
}
