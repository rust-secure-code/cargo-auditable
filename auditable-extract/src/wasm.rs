// Initial implementation of WASM parsing.
// It MAY perform heap allocations!
// TODO: guarantee zero-allocation parsing.

use crate::Error;

use wasmparser;

pub(crate) fn raw_auditable_data_wasm(input: &[u8]) -> Result<&[u8], Error> {
    let parser = wasmparser::Parser::new(0);

    for payload in parser.parse_all(&input) {
        // wasmparser errors are strings, so we can't reasonably convert them
        match payload.map_err(|_| Error::MalformedFile)? {
            wasmparser::Payload::CustomSection(reader) => {
                if reader.name() == ".dep-v0" {
                    return Ok(reader.data());
                }
            }
            // ignore everything that's not a custom section
            _ => {}
        }
    }
    Err(Error::NoAuditData)
}