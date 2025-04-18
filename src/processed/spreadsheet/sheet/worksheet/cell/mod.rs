pub mod cell_property;
pub mod cell_value;

use cell_property::CellProperty;
use cell_value::CellValueType;

use crate::common_types::Coordinate;

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub coordinate: Coordinate,
    pub value: CellValueType,
    pub property: CellProperty,
}

impl Cell {
    pub(crate) fn default(coordinate: Coordinate) -> Self {
        return Self {
            coordinate,
            value: CellValueType::Empty,
            property: CellProperty::default(),
        };
    }
}
