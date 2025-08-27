pub mod calculation_reference;
pub mod cell;
pub mod table;

#[cfg(feature = "serde")]
use serde::Serialize;

#[cfg(feature = "drawing")]
use std::collections::BTreeMap;

use anyhow::bail;
use std::{
    cmp::{max, min},
    u64,
};

#[cfg(feature = "drawing")]
use crate::processed::drawing::worksheet_drawing::{
    anchor_type::DrawingAnchorType, content_type::DrawingContentType, WorksheetDrawing,
};

#[cfg(feature = "drawing")]
use crate::raw::drawing::worksheet_drawing::{XlsxWorksheetDrawing, XlsxWorksheetDrawingType};

use calculation_reference::CalculationReferenceMode;
use cell::{cell_property::CellProperty, cell_value::CellValueType, Cell};
use table::Table;

use crate::{
    common_types::{Coordinate, Dimension},
    packaging::relationship::XlsxRelationships,
    processed::shared::hyperlink::Hyperlink,
    raw::{
        drawing::{scheme::color_scheme::XlsxColorScheme, theme::XlsxTheme},
        spreadsheet::{
            shared_string::shared_string_item::XlsxSharedStringItem,
            sheet::worksheet::{
                cell::XlsxCell, column_information::XlsxColumnInformation,
                hyperlink::XlsxHyperlink, row::XlsxRow, XlsxWorksheet,
            },
            stylesheet::{
                format::{
                    alignment::XlsxAlignment, cell_format::XlsxCellFormat,
                    protection::XlsxCellProtection,
                },
                XlsxStyleSheet,
            },
            table::XlsxTable,
            workbook::defined_name::XlsxDefinedNames,
        },
    },
};
use crate::processed::spreadsheet::sheet::worksheet::cell::cell_value::plain_text::PlainText;

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Worksheet {
    pub name: String,
    pub sheet_id: u64,

    /// None if the sheet does not contain any data
    pub dimension: Option<Dimension>,

    pub merged_cells: Vec<Dimension>,

    pub tables: Vec<Table>,

    /// Value that indicates whether to use a 1900 or 1904 date base when converting serial values in the workbook to dates.
    ///
    /// - true: workbook uses the 1904 backward compatibility date system.
    /// - false:  the workbook uses a date system based in 1900, as specified by the value of the dateCompatibility attribute.
    /// The default value for this attribute is false.
    pub is_1904: bool,

    /// Calculation Reference Mode
    pub calculation_reference_mode: CalculationReferenceMode,

    // private
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    raw_sheet: Box<XlsxWorksheet>,

    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    worksheet_rels: Box<XlsxRelationships>,

    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    shared_string_items: Box<Vec<XlsxSharedStringItem>>,

    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    stylesheet: Box<XlsxStyleSheet>,

    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    theme: Option<Box<XlsxTheme>>,

    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    defined_names: Box<XlsxDefinedNames>,

    #[cfg(feature = "drawing")]
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    drawing_rels: Box<XlsxRelationships>,

    #[cfg(feature = "drawing")]
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    raw_drawing: Option<Box<XlsxWorksheetDrawing>>,

    // (r_id, bytes)
    #[cfg(feature = "drawing")]
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    image_bytes: Box<BTreeMap<String, Vec<u8>>>,
}

impl Worksheet {
    /// get all cells within a worksheet.
    pub fn get_cells(&self) -> anyhow::Result<Vec<Cell>> {
        let mut cells: Vec<Cell> = vec![];

        let Some(dimension) = self.dimension else {
            return Ok(cells);
        };

        let (start, end) = (dimension.start, dimension.end);
        let mut row_index = start.row;

        while row_index <= end.row {
            let mut col_index = start.col;
            while col_index <= end.col {
                let cell = self.get_cell(Coordinate::from_point((row_index, col_index)))?;
                cells.push(cell);
                col_index += 1;
            }
            row_index += 1;
        }

        Ok(cells)
    }

    /// get cell value and styles for a specific coordinate.
    ///
    /// The style here ignoring table settings.
    /// If the cell is within a table that has different header row colors, column/row stripes, and etc.
    /// The appearance can be different.
    pub fn get_cell(&self, coordinate: Coordinate) -> anyhow::Result<Cell> {
        // if !self.coordinate_in_range(coordinate) {
        //     bail!(
        //         "Coordinate: {:?} is not within worksheet dimension.",
        //         coordinate
        //     )
        // }
        let Some(row) = self.get_raw_row(coordinate) else {
            return Ok(Cell::default(coordinate));
        };

        let Some(cell) = self.get_raw_cell(coordinate, row.clone()) else {
            return Ok(Cell::default(coordinate));
        };

        let color_scheme = self.get_color_scheme();

        let col = self.get_raw_col_info(coordinate);
        // let val = if let Some(value) = cell.cell_value {
        //     value.raw_value
        // } else { "".to_string() };

        // let cell_value = CellValueType::PlainText(PlainText {
        //     phonetic_properties: None,
        //     phonetic_runs: None,
        //     text: val,
        // });
        let cell_value = CellValueType::from_raw(
            &cell,
            *self.shared_string_items.clone(),
            *self.stylesheet.clone(),
            color_scheme.clone(),
        )?;

        let num_format_id = self.get_id(&cell, &row, col.clone(), &|x| {
            self.get_number_format_id_helper(x)
        });
        let fill_id = self.get_id(&cell, &row, col.clone(), &|x| {
            self.get_fill_id_helper(x)
        });
        let border_id = self.get_id(&cell, &row, col.clone(), &|x| {
            self.get_border_id_helper(x)
        });
        let font_id = self.get_id(&cell, &row, col.clone(), &|x| {
            self.get_font_id_helper(x)
        });
        let alignment = self.get_alignment(cell.clone(), row.clone(), col.clone());
        let protection = self.get_protection(cell.clone(), row.clone(), col.clone());

        let cell_property = CellProperty::from_raw(
            cell.clone(),
            row.clone(),
            col.clone(),
            fill_id,
            font_id,
            border_id,
            num_format_id,
            alignment,
            protection,
            self.get_hyperlink(coordinate),
            (*self.raw_sheet).sheet_format_properties.clone(),
            *self.stylesheet.clone(),
            color_scheme.clone(),
        );
        // println!("12222222222222222222222222222222222");
        Ok(Cell {
            coordinate,
            value: cell_value,
            property: cell_property,
        })
    }

    /// get all drawings within a worksheet.
    #[cfg(feature = "drawing")]
    pub fn get_drawings(&self) -> Vec<WorksheetDrawing> {
        let mut drawings: Vec<WorksheetDrawing> = vec![];
        let Some(raw_drawings) = self.raw_drawing.clone() else {
            return drawings;
        };
        for raw in raw_drawings.drawings.unwrap_or(vec![]).into_iter() {
            match raw {
                XlsxWorksheetDrawingType::AbsoluteAnchor(absolute_anchor) => {
                    let Some(content) = DrawingContentType::from_raw(
                        absolute_anchor.clone().drawing_content,
                        absolute_anchor.clone().extent,
                        absolute_anchor.clone().position,
                        absolute_anchor.clone().client_data,
                        None,
                        *self.drawing_rels.clone(),
                        *self.image_bytes.clone(),
                        *self.defined_names.clone(),
                        self.get_color_scheme(),
                        self.theme.clone(),
                    ) else {
                        continue;
                    };

                    let anchor = DrawingAnchorType::AbsoluteAnchor;

                    drawings.push(WorksheetDrawing { anchor, content });
                }
                XlsxWorksheetDrawingType::OneCellAnchor(one_cell_anchor_drawing) => {
                    let Some(content) = DrawingContentType::from_raw(
                        one_cell_anchor_drawing.clone().drawing_content,
                        one_cell_anchor_drawing.clone().extent,
                        None,
                        one_cell_anchor_drawing.clone().client_data,
                        None,
                        *self.drawing_rels.clone(),
                        *self.image_bytes.clone(),
                        *self.defined_names.clone(),
                        self.get_color_scheme(),
                        self.theme.clone(),
                    ) else {
                        continue;
                    };

                    let anchor =
                        DrawingAnchorType::from_one_cell_anchor(one_cell_anchor_drawing.clone());

                    drawings.push(WorksheetDrawing { anchor, content });
                }
                XlsxWorksheetDrawingType::TwoCellAnchor(two_cell_anchor_drawing) => {
                    let Some(content) = DrawingContentType::from_raw(
                        two_cell_anchor_drawing.clone().drawing_content,
                        None,
                        None,
                        two_cell_anchor_drawing.clone().client_data,
                        None,
                        *self.drawing_rels.clone(),
                        *self.image_bytes.clone(),
                        *self.defined_names.clone(),
                        self.get_color_scheme(),
                        self.theme.clone(),
                    ) else {
                        continue;
                    };

                    let anchor =
                        DrawingAnchorType::from_two_cell_anchor(two_cell_anchor_drawing.clone());

                    drawings.push(WorksheetDrawing { anchor, content });
                }
            }
        }

        return drawings;
    }
}

impl Worksheet {
    pub(crate) fn from_raw(
        name: String,
        sheet_id: u64,
        worksheet: Box<XlsxWorksheet>,
        worksheet_rels: Box<XlsxRelationships>,
        tables: Box<Vec<XlsxTable>>,
        defined_names: Box<XlsxDefinedNames>,
        is_1904: bool,
        calculation_reference_mode: Option<CalculationReferenceMode>,
        shared_string_items: Box<Vec<XlsxSharedStringItem>>,
        stylesheet: Box<XlsxStyleSheet>,
        theme: Option<Box<XlsxTheme>>,
        #[cfg(feature = "drawing")] drawing_rels: Box<XlsxRelationships>,
        #[cfg(feature = "drawing")] raw_drawing: Option<Box<XlsxWorksheetDrawing>>,
        #[cfg(feature = "drawing")] image_bytes: Box<BTreeMap<String, Vec<u8>>>,
    ) -> Self {
        let default_table_style_name = if let Some(style) = stylesheet.clone().table_styles {
            style.default_table_style
        } else {
            None
        };

        let tables: Vec<Table> = tables
            .into_iter()
            .map(|t| Table::from_raw(t, default_table_style_name.clone()))
            .collect();

        return Self {
            name,
            sheet_id,
            dimension: Self::get_dimension(*worksheet.clone()),
            merged_cells: worksheet.merge_cells.clone().unwrap_or(vec![]),
            tables,
            is_1904,
            calculation_reference_mode: calculation_reference_mode
                .unwrap_or(CalculationReferenceMode::default()),
            raw_sheet: worksheet,
            worksheet_rels,
            shared_string_items,
            stylesheet,
            theme,
            defined_names,
            #[cfg(feature = "drawing")]
            raw_drawing,
            #[cfg(feature = "drawing")]
            drawing_rels,
            #[cfg(feature = "drawing")]
            image_bytes,
        };
    }
}

impl Worksheet {
    fn get_hyperlink(&self, cell_coordinate: Coordinate) -> Option<Hyperlink> {
        let hyperlinks = self.raw_sheet.hyperlinks.clone().unwrap_or(vec![]);
        if hyperlinks.is_empty() {
            return None;
        }
        let target_link: Vec<XlsxHyperlink> = hyperlinks
            .into_iter()
            .filter(|h| h.r#ref == Some(cell_coordinate))
            .collect();
        let Some(target_link) = target_link.first() else {
            return None;
        };
        return Hyperlink::from_raw(
            target_link.clone(),
            *self.worksheet_rels.clone(),
            *self.defined_names.clone(),
        );
    }

    fn coordinate_in_range(&self, coordinate: Coordinate) -> bool {
        let Some(dimension) = self.dimension.clone() else {
            return false;
        };
        if !(dimension.start.row..=dimension.end.row).contains(&coordinate.row) {
            return false;
        };
        if !(dimension.start.col..=dimension.end.col).contains(&coordinate.col) {
            return false;
        };

        return true;
    }

    fn get_dimension(worksheet: XlsxWorksheet) -> Option<Dimension> {
        let worksheet_dimension = worksheet.dimension;
        let Some(data) = worksheet.sheet_data else {
            return worksheet_dimension;
        };

        let rows = data.rows.unwrap_or(vec![]);
        if rows.is_empty() {
            return None;
        }
        let first_row = rows[0].row_index.unwrap_or(1);
        let last_row = rows[rows.len() - 1].row_index.unwrap_or(rows.len() as u64);

        let mut first_col = u64::MAX;
        let mut last_col = u64::MIN;

        for row in rows {
            let cells = row.cells.unwrap_or(vec![]);
            if cells.is_empty() {
                continue;
            }
            let f = if let Some(c) = cells[0].coordinate {
                c.col
            } else {
                1
            };
            if f < first_col {
                first_col = f
            }
            let l = if let Some(c) = cells[cells.len() - 1].coordinate {
                c.col
            } else {
                cells.len() as u64
            };
            if l > last_col {
                last_col = l
            }
        }

        if first_col > last_col {
            return worksheet_dimension;
        }

        let Some(worksheet_dimension) = worksheet_dimension else {
            return Some(Dimension {
                start: Coordinate {
                    row: first_row,
                    col: first_col,
                },
                end: Coordinate {
                    row: last_row,
                    col: last_col,
                },
            });
        };

        return Some(Dimension {
            start: Coordinate {
                row: min(first_row, worksheet_dimension.start.row),
                col: min(first_col, worksheet_dimension.start.col),
            },
            end: Coordinate {
                row: max(last_row, worksheet_dimension.end.row),
                col: max(last_col, worksheet_dimension.end.col),
            },
        });
    }

    /// get cell alignment information
    fn get_protection(
        &self,
        cell: XlsxCell,
        row_info: XlsxRow,
        col_info: Option<XlsxColumnInformation>,
    ) -> Option<XlsxCellProtection> {
        if let Some(n) = cell.style {
            if let Some(protection) = self.get_protection_helper(n) {
                return Some(protection);
            }
        }

        if let Some(n) = row_info.style {
            if let Some(protection) = self.get_protection_helper(n) {
                return Some(protection);
            }
        }

        if let Some(col) = col_info {
            if let Some(n) = col.style {
                if let Some(protection) = self.get_protection_helper(n) {
                    return Some(protection);
                }
            }
        }

        return None;
    }

    /// get protection for a cellXfs' xf_id.
    ///
    /// None if not specified or applyFont is set to false
    fn get_protection_helper(&self, xf_id: u64) -> Option<XlsxCellProtection> {
        let Some(cell_format) = self.get_cell_format(xf_id) else {
            return None;
        };
        if cell_format.protection.is_some() && cell_format.apply_protection == Some(true) {
            return cell_format.protection;
        }

        // check if there is any reference to cellStyleXfs
        let Some(cell_style_format_id) = cell_format.xf_id else {
            if cell_format.protection.is_some() && cell_format.apply_protection.is_none() {
                return cell_format.protection;
            } else {
                return None;
            }
        };

        let Some(cell_style_format) = self.get_cell_style_format(cell_style_format_id) else {
            return None;
        };

        if cell_style_format.protection.is_some()
            && cell_style_format.apply_protection.unwrap_or(true) == true
        {
            return cell_style_format.protection;
        }

        return None;
    }

    /// get cell alignment information
    fn get_alignment(
        &self,
        cell: XlsxCell,
        row_info: XlsxRow,
        col_info: Option<XlsxColumnInformation>,
    ) -> Option<XlsxAlignment> {
        if let Some(n) = cell.style {
            if let Some(alignment) = self.get_alignment_helper(n) {
                return Some(alignment);
            }
        }

        if let Some(n) = row_info.style {
            if let Some(alignment) = self.get_alignment_helper(n) {
                return Some(alignment);
            }
        }

        if let Some(col) = col_info {
            if let Some(n) = col.style {
                if let Some(alignment) = self.get_alignment_helper(n) {
                    return Some(alignment);
                }
            }
        }

        return None;
    }

    /// get alignment for a cellXfs' xf_id.
    ///
    /// None if not specified or applyFont is set to false
    fn get_alignment_helper(&self, xf_id: u64) -> Option<XlsxAlignment> {
        let Some(cell_format) = self.get_cell_format(xf_id) else {
            return None;
        };
        if cell_format.alignment.is_some() && cell_format.apply_alignment == Some(true) {
            return cell_format.alignment;
        }

        // check if there is any reference to cellStyleXfs
        let Some(cell_style_format_id) = cell_format.xf_id else {
            if cell_format.alignment.is_some() && cell_format.apply_alignment.is_none() {
                return cell_format.alignment;
            } else {
                return None;
            }
        };

        let Some(cell_style_format) = self.get_cell_style_format(cell_style_format_id) else {
            return None;
        };

        if cell_style_format.alignment.is_some()
            && cell_style_format.apply_alignment.unwrap_or(true) == true
        {
            return cell_style_format.alignment;
        }

        return None;
    }

    /// get id for a cell
    ///
    /// helper function:
    /// * `get_fill_id_helper`
    /// * `get_border_id_helper`
    /// * `get_font_id_helper`
    /// * `get_number_format_id_helper`
    fn get_id(
        &self,
        cell: &XlsxCell,
        row_info: &XlsxRow,
        col_info: Option<XlsxColumnInformation>,
        helper_function: &dyn Fn(u64) -> Option<u64>,
    ) -> Option<u64> {
        if let Some(n) = cell.style {
            if let Some(id) = helper_function(n) {
                return Some(id);
            }
        }

        if let Some(n) = row_info.style {
            if let Some(id) = helper_function(n) {
                return Some(id);
            }
        }

        if let Some(col) = col_info {
            if let Some(n) = col.style {
                if let Some(id) = helper_function(n) {
                    return Some(id);
                }
            }
        }

        return None;
    }

    /// get border id for a cellXfs' xf_id.
    ///
    /// None if not specified or applyFont is set to false
    fn get_border_id_helper(&self, xf_id: u64) -> Option<u64> {
        let Some(cell_format) = self.get_cell_format(xf_id) else {
            return None;
        };
        if cell_format.border_id.is_some() && cell_format.apply_border == Some(true) {
            return cell_format.border_id;
        }

        // check if there is any reference to cellStyleXfs
        let Some(cell_style_format_id) = cell_format.xf_id else {
            if cell_format.border_id.is_some() && cell_format.apply_border.is_none() {
                return cell_format.border_id;
            } else {
                return None;
            }
        };

        let Some(cell_style_format) = self.get_cell_style_format(cell_style_format_id) else {
            return None;
        };

        if cell_style_format.border_id.is_some()
            && cell_style_format.apply_border.unwrap_or(true) == true
        {
            return cell_style_format.border_id;
        }

        return None;
    }

    /// get fill id for a cellXfs' xf_id.
    ///
    /// None if not specified or applyFill is set to false
    fn get_fill_id_helper(&self, xf_id: u64) -> Option<u64> {
        let Some(cell_format) = self.get_cell_format(xf_id) else {
            return None;
        };
        if cell_format.fill_id.is_some() && cell_format.apply_fill == Some(true) {
            return cell_format.fill_id;
        }

        // check if there is any reference to cellStyleXfs
        let Some(cell_style_format_id) = cell_format.xf_id else {
            if cell_format.fill_id.is_some() && cell_format.apply_fill.is_none() {
                return cell_format.fill_id;
            } else {
                return None;
            }
        };

        let Some(cell_style_format) = self.get_cell_style_format(cell_style_format_id) else {
            return None;
        };

        if cell_style_format.fill_id.is_some()
            && cell_style_format.apply_fill.unwrap_or(true) == true
        {
            return cell_style_format.fill_id;
        }

        return None;
    }

    /// get font id for a cellXfs' xf_id.
    ///
    /// None if not specified or applyFont is set to false
    fn get_font_id_helper(&self, xf_id: u64) -> Option<u64> {
        let Some(cell_format) = self.get_cell_format(xf_id) else {
            return None;
        };
        if cell_format.font_id.is_some() && cell_format.apply_font == Some(true) {
            return cell_format.font_id;
        }

        // check if there is any reference to cellStyleXfs
        let Some(cell_style_format_id) = cell_format.xf_id else {
            if cell_format.font_id.is_some() && cell_format.apply_font.is_none() {
                return cell_format.font_id;
            } else {
                return None;
            }
        };

        let Some(cell_style_format) = self.get_cell_style_format(cell_style_format_id) else {
            return None;
        };

        if cell_style_format.font_id.is_some()
            && cell_style_format.apply_font.unwrap_or(true) == true
        {
            return cell_style_format.font_id;
        }

        return None;
    }

    /// get number format id for a cellXfs' xf_id.
    ///
    /// None if not specified or apply_number_format is set to false
    fn get_number_format_id_helper(&self, xf_id: u64) -> Option<u64> {
        let Some(cell_format) = self.get_cell_format(xf_id) else {
            return None;
        };
        if cell_format.num_fmt_id.is_some() && cell_format.apply_number_format == Some(true) {
            return cell_format.num_fmt_id;
        }

        // check if there is any reference to cellStyleXfs
        let Some(cell_style_format_id) = cell_format.xf_id else {
            if cell_format.num_fmt_id.is_some() && cell_format.apply_number_format.is_none() {
                return cell_format.num_fmt_id;
            } else {
                return None;
            }
        };

        let Some(cell_style_format) = self.get_cell_style_format(cell_style_format_id) else {
            return None;
        };

        if cell_style_format.num_fmt_id.is_some()
            && cell_style_format.apply_number_format.unwrap_or(true) == true
        {
            return cell_style_format.num_fmt_id;
        }

        return None;
    }

    fn get_raw_cell(&self, coordinate: Coordinate, row: XlsxRow) -> Option<XlsxCell> {
        row.cells.as_ref()?.get(coordinate.col as usize).cloned()
        // // let cells = row.cells.unwrap_or(vec![]);
        //
        // let raw_cell: Vec<XlsxCell> = cells
        //     .into_iter()
        //     .filter(|c| c.coordinate == Some(coordinate))
        //     .collect();
        //
        // return raw_cell.first().cloned();
    }

    fn get_raw_col_info(&self, coordinate: Coordinate) -> Option<XlsxColumnInformation> {
        let cols = self.raw_sheet.column_infos.clone().unwrap_or(vec![]);

        let col_coordintae = coordinate.col;

        for col in cols {
            let min = col.min_column.unwrap_or(u64::MIN);
            let max = col.max_column.unwrap_or(u64::MAX);
            if (min..=max).contains(&col_coordintae) {
                return Some(col);
            }
        }

        return None;
    }

    fn get_raw_row(&self, coordinate: Coordinate) -> Option<XlsxRow> {
        let sheet_data = self.raw_sheet.sheet_data.as_ref()?;
        sheet_data.rows.as_ref()?.get(coordinate.row as usize).cloned()
        // self.raw_sheet.sheet_data?.rows?.get(coordinate.row as usize).cloned()
        // let Some(sheet_data) = self.raw_sheet.clone().sheet_data else {
        //     return None;
        // };
        //
        // // let rows = sheet_data.rows.unwrap_or(vec![]);
        // // rows.get(coordinate.row)
        // // let row: Vec<XlsxRow> = rows
        // //     .into_iter()
        // //     .filter(|r| r.row_index == Some(coordinate.row))
        // //     .collect();
        // //
        // // return row.first().cloned();
        // sheet_data.rows?.get(coordinate.row as usize).cloned()
    }

    fn get_cell_format(&self, xf_id: u64) -> Option<XlsxCellFormat> {
        let Ok(xf_id) = TryInto::<usize>::try_into(xf_id) else {
            return None;
        };

        return self.stylesheet.get_cell_format(xf_id);
    }

    fn get_cell_style_format(&self, xf_id: u64) -> Option<XlsxCellFormat> {
        let Ok(cell_style_format_id) = TryInto::<usize>::try_into(xf_id) else {
            return None;
        };

        return self.stylesheet.get_cell_style_format(cell_style_format_id);
    }

    fn get_color_scheme(&self) -> Option<XlsxColorScheme> {
        let mut color_scheme: Option<XlsxColorScheme> = None;

        if let Some(theme) = self.theme.clone() {
            if let Some(theme_elements) = theme.theme_elements {
                color_scheme = theme_elements.color_scheme
            }
        };
        return color_scheme;
    }
}
