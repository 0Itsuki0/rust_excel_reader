#[cfg(feature = "serde")]
use serde::Serialize;

use crate::{common_types::Text, raw::spreadsheet::string_item::phonetic_run::XlsxPhoneticRun};

/// https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.phoneticrun?view=openxml-3.0.1
///
/// This element represents a run of text which displays a phonetic hint for this String Item (si).
///
/// Example
/// ```
/// <si>
///     <t>課きく　毛こ</t>
///     <rPh sb="0" eb="1">
///         <t>カ</t>
///     </rPh>
///     <rPh sb="4" eb="5">
///        <t>ケ</t>
///     </rPh>
///     <phoneticPr fontId="1"/>
/// </si>
/// ```
///
/// The above example shows a String Item that displays some Japanese text "課きく　毛こ."
/// It also displays some phonetic text across the top of the cell.
/// The phonetic text character, "カ" is displayed over the "課" character and the phonetic text "ケ" is displayed above the "毛" character, using the font record in the style sheet at index 1.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct PhoneticRun {
    pub text: Text,

    /// An integer used as a zero-based index representing the starting offset into the base text for this phonetic run.
    /// This represents the starting point in the base text the phonetic hint applies to.
    pub base_text_start_index: u64,

    /// An integer used as a zero-based index representing the ending offset into the base text for this phonetic run.
    /// This represents the ending point in the base text the phonetic hint applies to.
    /// This value shall be between 0 and the total length of the base text.
    /// The following condition shall be true: sb < eb.
    pub base_text_end_index: u64,
}

impl PhoneticRun {
    pub(crate) fn from_raw(run: &XlsxPhoneticRun) -> Option<Self> {
        if run.text.is_none() || run.base_text_end_index.is_none() || run.base_text_start_index.is_none() { return None; }
        // let (Some(t), Some(s), Some(e)) = (run.text, run.base_text_start_index, run.base_text_end_index)
        // else {
        //     return None;
        // };
        Some(Self {
            text: run.text.clone().unwrap(),
            base_text_start_index: run.base_text_start_index?,
            base_text_end_index: run.base_text_end_index?,
        })
    }
}
