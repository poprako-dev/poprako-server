use super::*;

// import_query(ImportTermbaseQuery)(positive): force_merge defaults to false.
// native_wire(ExportTermbaseVal)(positive): exported JSON deserializes as the independent import Instr.

use axum::extract::Query;

use crate::data::view::termbase_port::TermbaseTermView;

#[test]
fn import_query_defaults_force_merge_to_false() {
    //
    let uri = "http://localhost/termbases/import".parse().unwrap();

    let Query(query) =
        Query::<ImportTermbaseQuery>::try_from_uri(&uri).unwrap();

    assert!(!query.force_merge);
}

#[test]
fn native_export_wire_deserializes_as_import_instr() {
    //
    let export_termbase_val = ExportTermbaseVal {
        name: "Glossary".into(),
        description: None,
        terms: vec![TermbaseTermView {
            source: "Source".into(),
            targets: vec!["Target".into()],
            comment: None,
        }],
    };

    let json = serde_json::to_value(export_termbase_val).unwrap();

    let instr = serde_json::from_value::<ImportTermbaseInstr>(json).unwrap();

    assert_eq!(instr.name, "Glossary");

    assert_eq!(instr.terms[0].source, "Source");
}
