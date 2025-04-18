/// https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.referencemodevalues?view=openxml-3.0.1
///
/// * A1
/// * R1C1
#[derive(Debug, Clone, PartialEq)]
pub enum CalculationReferenceMode {
    A1,
    R1C1,
}

impl CalculationReferenceMode {
    pub(crate) fn from_string(s: Option<String>) -> Option<Self> {
        let Some(s) = s else { return None };
        return match s.as_ref() {
            "A1" => Some(Self::A1),
            "R1C1" => Some(Self::R1C1),
            _ => None,
        };
    }
}
