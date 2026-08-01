use std::collections::HashMap;
use std::fmt::Write as _;

use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;

/// Chapter export formatting rules.
pub struct ChapterExportComplex;

impl ChapterExportComplex {
    /// Converts pages and units into LabelPlus text.
    pub fn make_label_plus(
        pages: &[PageInfo],
        units_by_page_id: &HashMap<String, Vec<UnitInfo>>,
    ) -> String {
        //
        let mut output = String::new();

        output.push_str("1,0\n");

        output.push_str("-\n");

        output.push_str("框内\n");

        output.push_str("框外\n");

        output.push_str("-\n");

        output.push_str("Exported by PopRaKo Web\n");

        for page_info in pages {
            //
            let image_name = label_plus_image_name(page_info);

            // FIXME: why ignore? and similar ones.
            write!(output, "\n\n>>>>>>>>[{}]<<<<<<<<\n", image_name).unwrap_or_else(|error| {
                tracing::error!(
                    err = %error,
                    "[ChapterExportComplex::make_label_plus] failed to write page header",
                );
            });

            let units = units_by_page_id
                .get(&page_info.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);

            for (index, unit_info) in units.iter().enumerate() {
                //
                let group = match unit_info.is_bubble {
                    //
                    true => 1,

                    false => 2,
                };

                writeln!(
                    output,
                    "----------------[{}]----------------[{:.4},{:.4},{}]",
                    index + 1,
                    unit_info.coord.x_coord,
                    unit_info.coord.y_coord,
                    group
                )
                .unwrap_or_else(|error| {
                    tracing::error!(
                        err = %error,
                        "[ChapterExportComplex::make_label_plus] failed to write unit line",
                    );
                });

                if let Some(main_text) = select_main_text(unit_info) {
                    //
                    output.push_str(main_text);

                    output.push('\n');
                }

                output.push('\n');
            }
        }

        output
    }
}

// Build a LabelPlus image filename from a page's stored index and image
// file extension (defaults to `jpg` when the image key has no extension).
fn label_plus_image_name(page_info: &PageInfo) -> String {
    format!("{:03}.{}", page_info.index, page_info.image_ext.suffix())
}

// Return the proofread text if non-empty, falling back to translated text
// if no proofread content is available.
fn select_main_text(unit_info: &UnitInfo) -> Option<&str> {
    unit_info
        .proofread_text
        .as_deref()
        .filter(|text| !text.is_empty())
        .or_else(|| {
            unit_info
                .translated_text
                .as_deref()
                .filter(|text| !text.is_empty())
        })
}
