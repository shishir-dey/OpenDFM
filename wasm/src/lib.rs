use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GerberAnalysis {
    file_name: String,
    byte_length: usize,
    layers: Vec<GerberLayer>,
    violations: Vec<DfmViolation>,
    board: BoardSummary,
    svg: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GerberLayer {
    name: String,
    kind: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DfmViolation {
    rule: String,
    count: usize,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoardSummary {
    width_mm: f64,
    height_mm: f64,
    layer_count: usize,
    hole_count: usize,
    pad_count: usize,
}

fn empty_analysis(file_name: &str, byte_length: usize) -> GerberAnalysis {
    GerberAnalysis {
        file_name: file_name.to_owned(),
        byte_length,
        layers: Vec::new(),
        violations: Vec::new(),
        board: BoardSummary::default(),
        svg: None,
    }
}

/// Entry point reserved for Gerber parsing and DFM analysis.
///
/// The boilerplate currently records the input identity and returns empty
/// layers, violations, board measurements, and SVG output.
#[wasm_bindgen]
pub fn analyze_gerber(file_name: &str, source: &[u8]) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&empty_analysis(file_name, source.len()))
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_an_empty_analysis_shell() {
        let result = empty_analysis("board.gbr", 12);

        assert_eq!(result.file_name, "board.gbr");
        assert_eq!(result.byte_length, 12);
        assert!(result.layers.is_empty());
        assert!(result.violations.is_empty());
        assert!(result.svg.is_none());
        assert_eq!(result.board.width_mm, 0.0);
        assert_eq!(result.board.height_mm, 0.0);
        assert_eq!(result.board.layer_count, 0);
        assert_eq!(result.board.hole_count, 0);
        assert_eq!(result.board.pad_count, 0);
    }
}
