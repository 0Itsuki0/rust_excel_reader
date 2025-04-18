use crate::raw::drawing::color::XlsxColorEnum;

/// https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.alphainverse?view=openxml-3.0.1
///
/// Example:
/// ```
/// <a:alphaInv>
///     <a:schemeClr val="phClr" />
/// </a:alphaInv>
/// ```
pub type XlsxAlphaInverse = XlsxColorEnum;
