use anyhow::bail;

#[cfg(feature = "serde")]
use serde::Serialize;

use error_value::CellErrorType;
use formula::Formula;
use phonetic_properties::PhoneticProperties;
use phonetic_run::PhoneticRun;
use plain_text::PlainText;
use rich_text::{RichText, RichTextRun};

use super::cell_property::font::Font;
use crate::{
    common_types::Text,
    helper::string_to_bool,
    raw::{
        drawing::scheme::color_scheme::XlsxColorScheme,
        spreadsheet::{
            shared_string::shared_string_item::XlsxSharedStringItem,
            sheet::worksheet::cell::XlsxCell, string_item::XlsxStringItem,
            stylesheet::XlsxStyleSheet,
        },
    },
};

pub mod error_value;
pub mod formula;
pub mod phonetic_properties;
pub mod phonetic_run;
pub mod plain_text;
pub mod rich_text;

/// ST_CellType: https://c-rex.net/samples/ooxml/e1/Part4/OOXML_P4_DOCX_ST_CellType_topic_ID0E6NEFB.html
///
/// Different data types that can appear as a value in a worksheet cell
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CellValueType {
    Numeric(f64),
    /// Rich inline String or shared string
    RichText(RichText),
    /// Formula
    Formula(Formula),
    /// plain inline String or shared string
    PlainText(PlainText),
    /// Boolean
    Bool(bool),
    /// Date, Time or DateTime in ISO 8601
    DateTime(Text),
    /// Error
    Error(CellErrorType),
    /// Empty cell
    #[default]
    Empty,
}

impl CellValueType {
    pub(crate) fn from_raw(
        cell: &XlsxCell,
        shared_string_items: Vec<XlsxSharedStringItem>,
        stylesheet: XlsxStyleSheet,
        color_scheme: Option<XlsxColorScheme>,
    ) -> anyhow::Result<Self> {
        if cell.formula.is_none() && cell.inline_string.is_none() && cell.cell_value.is_none() {
            return Ok(Self::Empty);
        }

        // inline string
        if let Some(ref is) = cell.inline_string {
            return Self::from_string_item(is, stylesheet.clone(), color_scheme.clone());
        }

        // formula
        if let Some(ref f) = cell.formula {
            let v = if let Some(ref cell_value) = cell.cell_value {
                Some(cell_value.raw_value.clone())
            } else {
                None
            };
            return Ok(Self::Formula(Formula {
                formula: f.raw_value.clone(),
                last_calculated_value: v,
            }));
        }

        if let Some(ref v) = cell.cell_value {
            if v.raw_value.is_empty() {
                return Ok(Self::Empty);
            }
            if cell.r#type.is_none() {
                return Ok(Self::from_numeric_string(&v.raw_value));
            }
            return match cell.r#type.as_ref().unwrap().as_str() {
                "b" => Ok(Self::Bool(string_to_bool(&v.raw_value).unwrap_or(true))),
                "d" => Ok(Self::DateTime(v.raw_value.clone())),
                "n" => Ok(Self::from_numeric_string(&v.raw_value)),
                "e" => Ok(Self::Error(CellErrorType::from_string(&v.raw_value)?)),
                // shared string
                "s" => {
                    let index: usize = v.raw_value.parse()?;
                    if index >= shared_string_items.len() {
                        bail!("Shared string index out of range.")
                    }
                    // let string_item = shared_string_items[index].clone();
                    Self::from_string_item(&shared_string_items[index], stylesheet.clone(), color_scheme.clone())
                }
                // formula string
                "str" => Ok(Self::from_numeric_string(&v.raw_value)), //bail!("cell has type str (formula) without <f> elements"),
                // inline string
                "is" | "inlineStr" => bail!("cell has type inline string without <is> elements"),
                t => {
                    bail!("unknown type {} on cell.", t)
                }
            };
        }

        return Ok(Self::Empty);
    }

    fn from_string_item(
        string_item: &XlsxStringItem,
        stylesheet: XlsxStyleSheet,
        color_scheme: Option<XlsxColorScheme>,
    ) -> anyhow::Result<Self> {
        let phonetic_runs: Option<Vec<PhoneticRun>> =
            if let Some(ref raw_run) = string_item.phonetic_run {
                if raw_run.is_empty() {
                    None
                } else {
                    Some(
                        raw_run
                            .into_iter()
                            .map(|r| PhoneticRun::from_raw(r))
                            .filter(|r| r.is_some())
                            .map(|r| r.unwrap())
                            .collect(),
                    )
                }
            } else {
                None
            };

        let mut phonetic_properties: Option<PhoneticProperties> = None;
        if phonetic_runs.is_some() && !phonetic_runs.clone().unwrap().is_empty() {
            if let Some(ref ph_pr) = string_item.phonetic_properties {
                phonetic_properties = Some(PhoneticProperties::from_raw(
                    ph_pr,
                    stylesheet.clone(),
                    color_scheme.clone(),
                ))
            }
        }

        // plain
        if let Some(ref t) = string_item.text {
            let plain = PlainText {
                phonetic_properties,
                phonetic_runs,
                text: t.clone(),
            };
            return Ok(Self::PlainText(plain));
        }

        // rich
        if let Some(ref raw_runs) = string_item.rich_text_run {
            if raw_runs.is_empty() {
                return Ok(Self::Empty);
            }
            let mut runs: Vec<RichTextRun> = vec![];
            for raw_run in raw_runs {
                let Some(ref t) = raw_run.text else {
                    continue;
                };
                let font = Font::from_raw_run_properties(
                    raw_run.run_properties.clone(),
                    stylesheet.clone().colors,
                    color_scheme.clone(),
                );
                runs.push(RichTextRun { font, text: t.clone() });
            }
            if runs.is_empty() {
                return Ok(Self::Empty);
            } else {
                return Ok(Self::RichText(RichText {
                    phonetic_properties,
                    phonetic_runs,
                    runs,
                }));
            }
        }

        bail!("Reading inline string without runs/plain text.")
    }

    fn from_numeric_string(s: &str) -> Self {
        if let Ok(f) = s.parse::<f64>() {
            return Self::Numeric(f);
        } else {
            return Self::PlainText(PlainText {
                phonetic_properties: None,
                phonetic_runs: None,
                text: s.to_string(),
            });
        }
    }
}
