use crate::shape::write::page_source::PageSourceRawIdentsReplShape;

// PageSourceRawIdentsReplShape(PageSourceRawIdentsReplShape)(positive): the replacement moves its vector and borrows page identifiers and filenames.
#[test]
fn page_source_replacement_moves_reference_vector_without_copying_strings() {
    let page_source_id = "page".to_owned();

    let raw_ident = "original.png".to_owned();

    let idents = vec![(page_source_id.as_str(), Some(raw_ident.as_str()))];

    let pointer = idents.as_ptr();

    let replacement = PageSourceRawIdentsReplShape { idents };

    assert_eq!(replacement.idents.as_ptr(), pointer);

    for (id, filename) in &replacement.idents {
        assert_eq!(id.as_ptr(), page_source_id.as_ptr());

        assert_eq!(filename.map(str::as_ptr), Some(raw_ident.as_ptr()));
    }
}
