use std::collections::HashMap;

use crate::model::{ParsedLayer, Point, Primitive};

#[derive(Clone, Copy)]
struct DrillFormat {
    integer: usize,
    decimal: usize,
    trailing_zero_omission: bool,
}

impl Default for DrillFormat {
    fn default() -> Self {
        Self {
            integer: 2,
            decimal: 4,
            trailing_zero_omission: false,
        }
    }
}

pub(crate) fn parse_drill(source: &str) -> ParsedLayer {
    let mut output = ParsedLayer::default();
    let mut format = DrillFormat::default();
    let mut unit_scale = 25.4;
    let mut tools = HashMap::<u32, f64>::new();
    let mut current_tool = None;
    let mut current = Point::default();
    let mut incremental = false;
    let mut explicit_format = false;

    for raw_line in source.lines() {
        let upper = raw_line.trim().to_ascii_uppercase();
        if upper.is_empty() {
            continue;
        }
        if let Some(format_text) = upper.strip_prefix(";FILE_FORMAT=") {
            if let Some((integer, decimal)) = format_text.split_once(':') {
                format.integer = integer.parse().unwrap_or(format.integer);
                format.decimal = decimal.parse().unwrap_or(format.decimal);
                explicit_format = true;
            }
            continue;
        }
        let line = upper.split(';').next().unwrap_or("").trim();
        if line.is_empty() || line == "M48" || line == "%" {
            continue;
        }
        if line.starts_with("METRIC") || line == "M71" {
            unit_scale = 1.0;
            if !explicit_format {
                format.integer = 3;
                format.decimal = 3;
            }
            format.trailing_zero_omission = line.contains("LZ");
            continue;
        }
        if line.starts_with("INCH") || line == "M72" {
            unit_scale = 25.4;
            if !explicit_format {
                format.integer = 2;
                format.decimal = 4;
            }
            format.trailing_zero_omission = line.contains("LZ");
            continue;
        }
        if line == "G90" {
            incremental = false;
            continue;
        }
        if line == "G91" {
            incremental = true;
            continue;
        }
        if line.starts_with('T') && line.contains('C') {
            let code_end = line[1..].bytes().take_while(u8::is_ascii_digit).count() + 1;
            let code = line[1..code_end].parse::<u32>().unwrap_or(0);
            let diameter = line[code_end..]
                .find('C')
                .and_then(|index| line[code_end + index + 1..].parse::<f64>().ok());
            if let Some(diameter) = diameter {
                tools.insert(code, diameter * unit_scale);
            }
            continue;
        }
        if line.starts_with('T') && line[1..].bytes().all(|byte| byte.is_ascii_digit()) {
            current_tool = line[1..].parse::<u32>().ok();
            continue;
        }
        if matches!(line, "M30" | "M00" | "M02") {
            break;
        }

        if let Some((start_text, end_text)) = line.split_once("G85") {
            let start = coordinate(start_text, current, format, unit_scale, incremental);
            let end = coordinate(end_text, start, format, unit_scale, incremental);
            let diameter = current_tool
                .and_then(|tool| tools.get(&tool).copied())
                .unwrap_or(0.3);
            output.add_primitive(Primitive::Line {
                start,
                end,
                width: diameter,
            });
            output.hole_count += 1;
            output.min_hole_diameter_mm = Some(
                output
                    .min_hole_diameter_mm
                    .map_or(diameter, |value| value.min(diameter)),
            );
            current = end;
            continue;
        }

        if line.contains('X') || line.contains('Y') {
            current = coordinate(line, current, format, unit_scale, incremental);
            let diameter = current_tool
                .and_then(|tool| tools.get(&tool).copied())
                .unwrap_or(0.3);
            output.add_primitive(Primitive::Drill {
                at: current,
                diameter,
            });
        }
    }

    output
}

fn coordinate(
    line: &str,
    current: Point,
    format: DrillFormat,
    unit_scale: f64,
    incremental: bool,
) -> Point {
    let x = field(line, 'X').map(|value| parse_number(value, format) * unit_scale);
    let y = field(line, 'Y').map(|value| parse_number(value, format) * unit_scale);
    Point {
        x: match (x, incremental) {
            (Some(value), true) => current.x + value,
            (Some(value), false) => value,
            (None, _) => current.x,
        },
        y: match (y, incremental) {
            (Some(value), true) => current.y + value,
            (Some(value), false) => value,
            (None, _) => current.y,
        },
    }
}

fn field(line: &str, field: char) -> Option<&str> {
    let index = line.find(field)? + 1;
    let bytes = line.as_bytes();
    let mut end = index;
    if end < bytes.len() && matches!(bytes[end], b'+' | b'-') {
        end += 1;
    }
    while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
        end += 1;
    }
    (end > index).then_some(&line[index..end])
}

fn parse_number(value: &str, format: DrillFormat) -> f64 {
    if value.contains('.') {
        return value.parse::<f64>().unwrap_or(0.0);
    }
    let negative = value.starts_with('-');
    let unsigned = value.trim_start_matches(['+', '-']);
    let number = unsigned.parse::<i64>().unwrap_or(0) as f64;
    if format.trailing_zero_omission {
        let fractional_digits = unsigned.len().saturating_sub(format.integer);
        let value = number / 10_f64.powi(fractional_digits as i32);
        return if negative { -value } else { value };
    }
    let number = number / 10_f64.powi(format.decimal as i32);
    if negative {
        -number
    } else {
        number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metric_excellon_holes() {
        let source = "M48\nMETRIC,LZ\nT01C0.800\n%\nT01\nX010000Y005000\nX020000Y015000\nM30\n";
        let layer = parse_drill(source);
        assert_eq!(layer.hole_count, 2);
        assert_eq!(layer.min_hole_diameter_mm, Some(0.8));
        assert!((layer.bounds.max_x - 20.4).abs() < 0.001);
    }
}
