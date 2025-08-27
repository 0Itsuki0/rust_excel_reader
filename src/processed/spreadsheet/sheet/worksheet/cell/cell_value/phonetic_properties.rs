#[cfg(feature = "serde")]
use serde::Serialize;

use crate::{
    processed::spreadsheet::sheet::worksheet::cell::cell_property::font::Font,
    raw::{
        drawing::scheme::color_scheme::XlsxColorScheme,
        spreadsheet::{
            string_item::phonetic_properties::XlsxPhoneticProperties, stylesheet::XlsxStyleSheet,
        },
    },
};

/// https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.phoneticproperties?view=openxml-3.0.1
///
/// This element represents a collection of phonetic properties that affect the display of phonetic text for this String Item (si).
/// Phonetic text is used to give hints as to the pronunciation of an East Asian language, and the hints are displayed as text within the spreadsheet cells across the top portion of the cell.
/// Since the phonetic hints are text, every phonetic hint is expressed as a phonetic run (rPh), and these properties specify how to display that phonetic run.
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
// tag: phoneticPr
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct PhoneticProperties {
    // Attributes
    /// Specifies how the text for the phonetic run is aligned across the top of the cells, with respect to the main text in the body of the cell.
    pub alignment: PhoneticAlignmentValue,

    /// An integer that is a zero-based index into the font record in the style sheet.
    /// Represents the font to be used to display this phonetic run.
    ///
    /// If this index is out of bounds, then the default font of the Normal style should be used in its place.
    /// This default font should be at index 0.
    pub font: Font,

    /// An enumeration which specifies which East Asian character set should be used to display the phonetic run
    ///
    pub r#type: PhoneticTypeValue,
}

impl PhoneticProperties {
    pub(crate) fn from_raw(
        properties: &XlsxPhoneticProperties,
        stylesheet: XlsxStyleSheet,
        color_scheme: Option<XlsxColorScheme>,
    ) -> Self {
        let font_id = properties.font_id.unwrap_or(0).try_into().unwrap_or(0);
        let raw_font = stylesheet.get_font(font_id);
        let font = Font::from_raw_font(raw_font, stylesheet.colors, color_scheme);

        return Self {
            alignment: PhoneticAlignmentValue::from_string(properties.alignment.clone()),
            font,
            r#type: PhoneticTypeValue::from_string(properties.r#type.clone()),
        };
    }
}

/// https://c-rex.net/samples/ooxml/e1/Part4/OOXML_P4_DOCX_ST_PhoneticType_topic_ID0EKVRFB.html
///
/// * halfwidthKatakana
/// * fullwidthKatakana
/// * Hiragana
/// * noConversion
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PhoneticTypeValue {
    HalfWidthKatakana,
    FullWidthKatakana,
    Hiragana,
    NoConversion,
}

impl PhoneticTypeValue {
    pub(crate) fn from_string(s: Option<String>) -> Self {
        let Some(s) = s else {
            return Self::NoConversion;
        };
        return match s.as_ref() {
            "halfwidthKatakana" => Self::HalfWidthKatakana,
            "fullwidthKatakana" => Self::FullWidthKatakana,
            "Hiragana" => Self::Hiragana,
            "noConversion" => Self::NoConversion,
            _ => Self::NoConversion,
        };
    }
}

/// https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.phoneticalignmentvalues?view=openxml-3.0.1
///
/// * Center,
/// * Distributed,
/// * Left
/// * NoControl
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PhoneticAlignmentValue {
    Center,
    Distributed,
    Left,
    NoControl,
}

impl PhoneticAlignmentValue {
    pub(crate) fn from_string(s: Option<String>) -> Self {
        let Some(s) = s else {
            return Self::NoControl;
        };
        return match s.as_ref() {
            "center" => Self::Center,
            "distributed" => Self::Distributed,
            "left" => Self::Left,
            "noControl" => Self::NoControl,
            _ => Self::NoControl,
        };
    }
}
