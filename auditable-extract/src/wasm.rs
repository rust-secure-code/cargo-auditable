//! Implements WASM parsing

use crate::Error;

use wasmparser::{self, Chunk, Payload};

pub(crate) fn raw_auditable_data_wasm(mut input: &[u8]) -> Result<&[u8], Error> {
    let mut parser = wasmparser::Parser::new(0);

    // `wasmparser` relies on manually advancing the offset,
    // which potentially allows infinite loops if the logic is wrong somewhere.
    // Therefore, limit the maximum number of iterations to 10,000.
    // This is way more than any sane WASM blob will have,
    // and prevents infinite loops in case of such logic errors.
    for _i in 0..10_000 {
        // wasmparser errors are strings, so we can't reasonably convert them
        let payload = match parser
            .parse(input, true)
            .map_err(|_| Error::MalformedFile)?
        {
            // This shouldn't be possible because `eof` is always true.
            Chunk::NeedMoreData(_) => return Err(Error::MalformedFile),

            Chunk::Parsed { payload, consumed } => {
                // Guard against made-up "consumed" values that would cause a panic
                input = match input.get(consumed..) {
                    Some(input) => input,
                    None => return Err(Error::MalformedFile),
                };
                payload
            }
        };

        match payload {
            Payload::CustomSection(reader) => {
                if reader.name() == ".dep-v0" {
                    return Ok(reader.data());
                }
            }
            // We reached the end without seeing ".dep-v0" custom section
            Payload::End(_) => return Err(Error::NoAuditData),
            // ignore everything that's not a custom section
            _ => {}
        }
    }

    if cfg!(debug_assertions) {
        panic!("The parser has been running for more than 10k sections! Is it stuck?");
    } else {
        Err(Error::MalformedFile)
    }
}
