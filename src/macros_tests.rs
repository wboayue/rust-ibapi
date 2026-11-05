use crate::common::test_utils::source_scan::visit_production_sources;
use std::collections::HashMap;

/// `(enum name, [(variant, code)])` for every `impl_wire_code!` invocation in `contents`.
fn wire_code_tables(contents: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut tables = Vec::new();
    let mut lines = contents.lines();
    while let Some(line) = lines.next() {
        let Some(rest) = line.trim().strip_prefix("impl_wire_code!(") else {
            continue;
        };
        let name = rest.trim_end_matches('{').trim().to_string();
        let rows = lines
            .by_ref()
            .map(str::trim)
            .take_while(|l| *l != "});")
            .filter_map(|l| l.trim_end_matches(',').split_once(" = "))
            .map(|(v, c)| (v.to_string(), c.to_string()))
            .collect();
        tables.push((name, rows));
    }
    tables
}

/// Variant name → its joined `///` doc text, for `pub enum <name>` in `contents`.
fn variant_docs(contents: &str, name: &str) -> Option<HashMap<String, String>> {
    let header = format!("pub enum {name} {{");
    let body = contents.lines().skip_while(|l| l.trim() != header).skip(1);
    let mut docs = HashMap::new();
    let mut pending = Vec::new();
    for line in body {
        let line = line.trim();
        if line == "}" {
            return Some(docs);
        }
        if let Some(doc) = line.strip_prefix("///") {
            pending.push(doc.trim());
        } else if !line.starts_with("#[") && !line.is_empty() {
            let variant = line.split(['(', ',', ' ']).next().unwrap_or_default();
            docs.insert(variant.to_string(), pending.join(" "));
            pending.clear();
        }
    }
    None
}

/// Every variant's ``Wire code `N`.`` doc agrees with its `impl_wire_code!` row.
/// The docs are what docs.rs readers see; the table is what the code does, and
/// nothing else keeps the two in step.
#[test]
fn wire_code_docs_match_impl_wire_code_tables() {
    let mut checked = 0;
    visit_production_sources(&mut |path, contents| {
        for (name, rows) in wire_code_tables(contents) {
            assert!(!rows.is_empty(), "{}: impl_wire_code!({name}) parsed no rows", path.display());
            let docs = variant_docs(contents, &name).unwrap_or_else(|| panic!("{}: no `pub enum {name}` beside its impl_wire_code!", path.display()));
            for (variant, code) in rows {
                let doc = docs.get(&variant).unwrap_or_else(|| panic!("{name}::{variant} not found in the enum"));
                let expected = format!("Wire code `{code}`");
                assert!(doc.contains(&expected), "{name}::{variant}: doc should say {expected}, got {doc:?}");
            }
            checked += 1;
        }
    });
    assert!(checked > 0, "found no impl_wire_code! invocations; the scan is broken");
}
