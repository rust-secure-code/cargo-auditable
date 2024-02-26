use serde_json;

/// Accepts BOM in JSON and minifies it,
/// working around https://github.com/CycloneDX/cyclonedx-rust-cargo/issues/628
pub fn minify_bom(bom: &[u8]) -> String {
    let mut json: serde_json::Value = serde_json::from_slice(bom).unwrap();
    // clear the unnecessary toplevel fields
    let toplevel = json.as_object_mut().unwrap();
    toplevel.remove("version");
    toplevel.remove("serialNumber");
    // clear components field if empty
    if let Some(components) = toplevel.get_mut("dependencies") {
        let components = components.as_array().unwrap();
        if components.is_empty() {
            toplevel.remove("dependencies");
        }
    }
    // clear empty arrays in dependencies
    if let Some(deps) = toplevel.get_mut("dependencies") {
        let deps = deps.as_array_mut().unwrap();
        deps.iter_mut().for_each(|dependency| {
            if let Some(deps_array) = dependency.get("dependsOn") {
                let deps_array = deps_array.as_array().unwrap();
                if deps_array.is_empty() {
                    dependency.as_object_mut().unwrap().remove("dependsOn");
                }
            }
        });
    }
    // .to_string() writes the minified JSON, unlike .to_string_pretty()
    serde_json::to_string(&json).unwrap()
}
