mod drill;
mod gerber;
mod layer;
mod model;
mod svg;

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::layer::detect_layer;
use crate::model::{LayerBounds, ParsedLayer};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GerberAnalysis {
    file_name: String,
    byte_length: usize,
    layers: Vec<GerberLayer>,
    violations: Vec<DfmViolation>,
    board: BoardSummary,
    warnings: Vec<String>,
    svg: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GerberLayer {
    name: String,
    kind: String,
    kind_label: String,
    color: String,
    svg: String,
    bounds: LayerBounds,
    primitive_count: usize,
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
    minimum_track_width_mm: f64,
    minimum_hole_diameter_mm: f64,
}

fn analyze_source(file_name: &str, source: &[u8]) -> GerberAnalysis {
    let text = String::from_utf8_lossy(source);
    let identity = detect_layer(file_name, &text);
    let parsed = if identity.is_drill {
        drill::parse_drill(&text)
    } else {
        gerber::parse_gerber(&text)
    };
    build_analysis(file_name, source.len(), identity, parsed)
}

fn build_analysis(
    file_name: &str,
    byte_length: usize,
    identity: layer::LayerIdentity,
    parsed: ParsedLayer,
) -> GerberAnalysis {
    let board = BoardSummary {
        width_mm: parsed.bounds.width(),
        height_mm: parsed.bounds.height(),
        layer_count: usize::from(!identity.is_drill),
        hole_count: parsed.hole_count,
        pad_count: parsed.pad_count,
        minimum_track_width_mm: parsed.min_track_width_mm.unwrap_or(0.0),
        minimum_hole_diameter_mm: parsed.min_hole_diameter_mm.unwrap_or(0.0),
    };
    let svg = svg::render_svg_fragment(&parsed, identity.color);
    let layer = GerberLayer {
        name: file_name.to_owned(),
        kind: identity.kind.to_owned(),
        kind_label: identity.label.to_owned(),
        color: identity.color.to_owned(),
        svg,
        bounds: parsed.bounds.into(),
        primitive_count: parsed.primitives.len(),
    };

    GerberAnalysis {
        file_name: file_name.to_owned(),
        byte_length,
        layers: vec![layer],
        violations: Vec::new(),
        board,
        warnings: parsed.warnings,
        svg: None,
    }
}

/// Parse an RS-274X Gerber or Excellon drill file and return an SVG layer.
/// Layer purpose is inferred from the filename and extension.
#[wasm_bindgen]
pub fn analyze_gerber(file_name: &str, source: &[u8]) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&analyze_source(file_name, source))
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_renders_a_copper_layer() {
        let source =
            b"%FSLAX24Y24*%%MOMM*%%ADD10C,0.200*%D10*X000000Y000000D02*X100000Y000000D01*M02*";
        let result = analyze_source("board-F_Cu.gbr", source);

        assert_eq!(result.layers[0].kind, "top-copper");
        assert_eq!(result.layers[0].primitive_count, 1);
        assert!(result.layers[0].svg.contains("<line"));
        assert!(result.board.width_mm > 10.0);
        assert_eq!(result.board.minimum_track_width_mm, 0.2);
    }

    #[test]
    fn detects_and_renders_a_drill_layer() {
        let source = b"M48\nMETRIC,LZ\nT01C0.800\n%\nT01\nX010000Y005000\nM30\n";
        let result = analyze_source("board-PTH.drl", source);

        assert_eq!(result.layers[0].kind, "drill");
        assert!(result.layers[0].svg.contains("<circle"));
        assert_eq!(result.board.hole_count, 1);
        assert_eq!(result.board.minimum_hole_diameter_mm, 0.8);
    }
}
