use std::collections::HashMap;

use crate::model::{Aperture, ApertureShape, ParsedLayer, Point, Primitive};

#[derive(Clone, Copy)]
enum Interpolation {
    Linear,
    Clockwise,
    CounterClockwise,
}

#[derive(Clone, Copy)]
struct CoordinateFormat {
    x_integer: usize,
    x_decimal: usize,
    y_integer: usize,
    y_decimal: usize,
    trailing_zero_omission: bool,
    incremental: bool,
}

impl Default for CoordinateFormat {
    fn default() -> Self {
        Self {
            x_integer: 2,
            x_decimal: 4,
            y_integer: 2,
            y_decimal: 4,
            trailing_zero_omission: false,
            incremental: false,
        }
    }
}

struct Parser {
    format: CoordinateFormat,
    unit_scale: f64,
    apertures: HashMap<u32, Aperture>,
    macros: HashMap<String, Aperture>,
    active_macro: Option<MacroBuilder>,
    current_aperture: Option<u32>,
    current: Point,
    operation: u32,
    interpolation: Interpolation,
    region: Option<Vec<Point>>,
    dark_polarity: bool,
    output: ParsedLayer,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            format: CoordinateFormat::default(),
            unit_scale: 25.4,
            apertures: HashMap::new(),
            macros: HashMap::new(),
            active_macro: None,
            current_aperture: None,
            current: Point::default(),
            operation: 2,
            interpolation: Interpolation::Linear,
            region: None,
            dark_polarity: true,
            output: ParsedLayer::default(),
        }
    }
}

pub(crate) fn parse_gerber(source: &str) -> ParsedLayer {
    let mut parser = Parser::default();

    for command in commands(source) {
        parser.process(&command);
    }

    parser.finish_macro();
    parser.finish_region();
    parser.output
}

impl Parser {
    fn process(&mut self, command: &str) {
        let token = command.trim().to_ascii_uppercase();

        if self.active_macro.is_some() && is_macro_primitive(&token) {
            if let Some(builder) = &mut self.active_macro {
                builder.add_primitive(&token);
            }
            return;
        }
        if self.active_macro.is_some() {
            self.finish_macro();
        }
        if let Some(name) = token.strip_prefix("AM") {
            self.active_macro = Some(MacroBuilder::new(name));
            if !self
                .output
                .warnings
                .iter()
                .any(|warning| warning.contains("aperture macros"))
            {
                self.output.warnings.push(
                    "Gerber aperture macros are approximated from their primitive bounds."
                        .to_owned(),
                );
            }
            return;
        }

        if token.is_empty()
            || token.starts_with("G04")
            || token.starts_with("TF")
            || token.starts_with("TA")
            || token.starts_with("TD")
        {
            return;
        }

        if token.starts_with("FS") {
            self.parse_format(&token);
            return;
        }
        if token == "MOMM" {
            self.unit_scale = 1.0;
            return;
        }
        if token == "MOIN" {
            self.unit_scale = 25.4;
            return;
        }
        if token.starts_with("ADD") {
            self.parse_aperture(&token);
            return;
        }
        if token == "LPD" {
            self.dark_polarity = true;
            return;
        }
        if token == "LPC" {
            self.dark_polarity = false;
            if !self
                .output
                .warnings
                .iter()
                .any(|warning| warning.contains("clear polarity"))
            {
                self.output.warnings.push(
                    "Gerber clear polarity is currently omitted from the SVG preview.".to_owned(),
                );
            }
            return;
        }
        let fields = parse_fields(&token);
        let mut close_region = false;
        for value in field_values(&fields, 'G') {
            match value.parse::<u32>().unwrap_or(0) {
                1 => self.interpolation = Interpolation::Linear,
                2 => self.interpolation = Interpolation::Clockwise,
                3 => self.interpolation = Interpolation::CounterClockwise,
                36 => self.region = Some(vec![self.current]),
                37 => close_region = true,
                70 => self.unit_scale = 25.4,
                71 => self.unit_scale = 1.0,
                90 => self.format.incremental = false,
                91 => self.format.incremental = true,
                _ => {}
            }
        }
        if close_region {
            self.finish_region();
        }

        let d_code = last_field(&fields, 'D').and_then(|value| value.parse::<u32>().ok());
        if let Some(code) = d_code {
            if code >= 10 {
                self.current_aperture = Some(code);
                if last_field(&fields, 'X').is_none() && last_field(&fields, 'Y').is_none() {
                    return;
                }
            } else if code > 0 {
                self.operation = code;
            }
        }

        let has_coordinates =
            last_field(&fields, 'X').is_some() || last_field(&fields, 'Y').is_some();
        if !has_coordinates && d_code.is_none() {
            return;
        }

        let next = Point {
            x: self.coordinate(last_field(&fields, 'X'), true, self.current.x),
            y: self.coordinate(last_field(&fields, 'Y'), false, self.current.y),
        };

        match self.operation {
            1 => self.draw(next, &fields),
            2 => {
                self.current = next;
                if let Some(region) = &mut self.region {
                    region.clear();
                    region.push(next);
                }
            }
            3 => {
                self.current = next;
                if self.dark_polarity {
                    let aperture = self
                        .current_aperture
                        .and_then(|code| self.apertures.get(&code).cloned())
                        .unwrap_or_default();
                    self.output
                        .add_primitive(Primitive::Flash { at: next, aperture });
                }
            }
            _ => self.current = next,
        }
    }

    fn draw(&mut self, next: Point, fields: &[(char, String)]) {
        let start = self.current;
        self.current = next;

        if let Some(region) = &mut self.region {
            if region.last().copied() != Some(next) {
                region.push(next);
            }
            return;
        }
        if !self.dark_polarity {
            return;
        }

        let width = self
            .current_aperture
            .and_then(|code| self.apertures.get(&code))
            .map(|aperture| aperture.width.max(0.001))
            .unwrap_or(0.1);

        match self.interpolation {
            Interpolation::Linear => {
                self.output.add_primitive(Primitive::Line {
                    start,
                    end: next,
                    width,
                });
            }
            Interpolation::Clockwise | Interpolation::CounterClockwise => {
                let offset_x = self.offset(last_field(fields, 'I'), true);
                let offset_y = self.offset(last_field(fields, 'J'), false);
                let center = Point {
                    x: start.x + offset_x,
                    y: start.y + offset_y,
                };
                self.output.add_primitive(Primitive::Arc {
                    start,
                    end: next,
                    center,
                    width,
                    clockwise: matches!(self.interpolation, Interpolation::Clockwise),
                });
            }
        }
    }

    fn parse_format(&mut self, token: &str) {
        self.format.trailing_zero_omission = token.as_bytes().get(2) == Some(&b'T');
        self.format.incremental = token.as_bytes().get(3) == Some(&b'I');

        if let Some((integer, decimal)) = format_pair(token, 'X') {
            self.format.x_integer = integer;
            self.format.x_decimal = decimal;
        }
        if let Some((integer, decimal)) = format_pair(token, 'Y') {
            self.format.y_integer = integer;
            self.format.y_decimal = decimal;
        }
    }

    fn parse_aperture(&mut self, token: &str) {
        let rest = &token[3..];
        let code_len = rest.bytes().take_while(u8::is_ascii_digit).count();
        let Some(code) = rest
            .get(..code_len)
            .and_then(|value| value.parse::<u32>().ok())
        else {
            return;
        };
        let definition = &rest[code_len..];
        let (shape_name, parameters) = definition.split_once(',').unwrap_or((definition, "0.1"));
        if let Some(aperture) = self.macros.get(shape_name).cloned() {
            self.apertures.insert(code, aperture);
            return;
        }
        let values = parameters
            .split(['X', 'x'])
            .filter_map(|value| value.parse::<f64>().ok())
            .collect::<Vec<_>>();
        let width = (values.first().copied().unwrap_or(0.1) * self.unit_scale).max(0.001);
        let mut height = (values
            .get(1)
            .copied()
            .unwrap_or(values.first().copied().unwrap_or(0.1))
            * self.unit_scale)
            .max(0.001);
        let shape = match shape_name.chars().next().unwrap_or('C') {
            'C' => ApertureShape::Circle,
            'R' => ApertureShape::Rectangle,
            'O' => ApertureShape::Obround,
            'P' => {
                height = width;
                ApertureShape::Polygon {
                    vertices: values.get(1).copied().unwrap_or(6.0) as usize,
                    rotation_deg: values.get(2).copied().unwrap_or(0.0),
                }
            }
            _ => {
                if !self
                    .output
                    .warnings
                    .iter()
                    .any(|warning| warning.contains("macro aperture"))
                {
                    self.output
                        .warnings
                        .push("A macro aperture was approximated as a circle.".to_owned());
                }
                ApertureShape::Circle
            }
        };
        self.apertures.insert(
            code,
            Aperture {
                shape,
                width,
                height,
            },
        );
    }

    fn coordinate(&self, value: Option<&str>, is_x: bool, current: f64) -> f64 {
        let Some(value) = value else { return current };
        let parsed = parse_number(
            value,
            if is_x {
                self.format.x_integer
            } else {
                self.format.y_integer
            },
            if is_x {
                self.format.x_decimal
            } else {
                self.format.y_decimal
            },
            self.format.trailing_zero_omission,
        ) * self.unit_scale;
        if self.format.incremental {
            current + parsed
        } else {
            parsed
        }
    }

    fn offset(&self, value: Option<&str>, is_x: bool) -> f64 {
        value.map_or(0.0, |value| {
            parse_number(
                value,
                if is_x {
                    self.format.x_integer
                } else {
                    self.format.y_integer
                },
                if is_x {
                    self.format.x_decimal
                } else {
                    self.format.y_decimal
                },
                self.format.trailing_zero_omission,
            ) * self.unit_scale
        })
    }

    fn finish_region(&mut self) {
        if let Some(points) = self.region.take() {
            if self.dark_polarity && points.len() >= 3 {
                self.output.add_primitive(Primitive::Region { points });
            }
        }
    }

    fn finish_macro(&mut self) {
        let Some(builder) = self.active_macro.take() else {
            return;
        };
        let name = builder.name.clone();
        self.macros.insert(name, builder.aperture(self.unit_scale));
    }
}

struct MacroBuilder {
    name: String,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    empty: bool,
}

impl MacroBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
            empty: true,
        }
    }

    fn add_primitive(&mut self, token: &str) {
        if token.starts_with('$') {
            return;
        }
        let values = token
            .split(',')
            .filter_map(|value| value.trim().parse::<f64>().ok())
            .collect::<Vec<_>>();
        let Some(code) = values.first().map(|value| *value as u32) else {
            return;
        };
        if values.get(1).copied().unwrap_or(1.0) <= 0.0 {
            return;
        }

        match code {
            // Circle: exposure, diameter, center x/y, rotation.
            1 if values.len() >= 5 => {
                let radius = values[2].abs() / 2.0;
                self.add_bounds(
                    values[3] - radius,
                    values[4] - radius,
                    values[3] + radius,
                    values[4] + radius,
                );
            }
            // Vector line: exposure, width, start x/y, end x/y, rotation.
            20 if values.len() >= 7 => {
                let rotation = values.get(7).copied().unwrap_or(0.0);
                let start = rotate_point(values[3], values[4], rotation);
                let end = rotate_point(values[5], values[6], rotation);
                let radius = values[2].abs() / 2.0;
                self.add_bounds(
                    start.0.min(end.0) - radius,
                    start.1.min(end.1) - radius,
                    start.0.max(end.0) + radius,
                    start.1.max(end.1) + radius,
                );
            }
            // Center line: exposure, width, height, center x/y, rotation.
            21 if values.len() >= 6 => self.add_rotated_rectangle(
                values[2],
                values[3],
                values[4],
                values[5],
                values.get(6).copied().unwrap_or(0.0),
            ),
            // Lower-left line: exposure, width, height, lower-left x/y, rotation.
            22 if values.len() >= 6 => self.add_rotated_rectangle(
                values[2],
                values[3],
                values[4] + values[2] / 2.0,
                values[5] + values[3] / 2.0,
                values.get(6).copied().unwrap_or(0.0),
            ),
            // Outline: exposure, vertex count, coordinate pairs, rotation.
            4 if values.len() >= 6 => {
                let point_count = values[2].max(0.0) as usize + 1;
                let rotation_index = 3 + point_count * 2;
                let rotation = values.get(rotation_index).copied().unwrap_or(0.0);
                for point in values[3..values.len().min(rotation_index)].chunks_exact(2) {
                    let (x, y) = rotate_point(point[0], point[1], rotation);
                    self.add_bounds(x, y, x, y);
                }
            }
            // Polygon: exposure, vertices, center x/y, diameter, rotation.
            5 if values.len() >= 6 => {
                let radius = values[5].abs() / 2.0;
                self.add_bounds(
                    values[3] - radius,
                    values[4] - radius,
                    values[3] + radius,
                    values[4] + radius,
                );
            }
            _ => {}
        }
    }

    fn add_rotated_rectangle(
        &mut self,
        width: f64,
        height: f64,
        center_x: f64,
        center_y: f64,
        rotation_deg: f64,
    ) {
        let radians = rotation_deg.to_radians();
        let half_width = width.abs() / 2.0;
        let half_height = height.abs() / 2.0;
        let extent_x = radians.cos().abs() * half_width + radians.sin().abs() * half_height;
        let extent_y = radians.sin().abs() * half_width + radians.cos().abs() * half_height;
        let (center_x, center_y) = rotate_point(center_x, center_y, rotation_deg);
        self.add_bounds(
            center_x - extent_x,
            center_y - extent_y,
            center_x + extent_x,
            center_y + extent_y,
        );
    }

    fn add_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        if self.empty {
            self.min_x = min_x;
            self.min_y = min_y;
            self.max_x = max_x;
            self.max_y = max_y;
            self.empty = false;
            return;
        }
        self.min_x = self.min_x.min(min_x);
        self.min_y = self.min_y.min(min_y);
        self.max_x = self.max_x.max(max_x);
        self.max_y = self.max_y.max(max_y);
    }

    fn aperture(self, unit_scale: f64) -> Aperture {
        let (width, height) = if self.empty {
            (0.1, 0.1)
        } else {
            (
                (self.max_x - self.min_x).abs(),
                (self.max_y - self.min_y).abs(),
            )
        };
        Aperture {
            shape: ApertureShape::Rectangle,
            width: (width * unit_scale).max(0.001),
            height: (height * unit_scale).max(0.001),
        }
    }
}

fn rotate_point(x: f64, y: f64, rotation_deg: f64) -> (f64, f64) {
    let radians = rotation_deg.to_radians();
    (
        x * radians.cos() - y * radians.sin(),
        x * radians.sin() + y * radians.cos(),
    )
}

fn is_macro_primitive(token: &str) -> bool {
    token.contains(',')
        && token
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit() || character == '$')
}

fn commands(source: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut current = String::new();
    for character in source.chars() {
        match character {
            '*' => {
                let command = current.trim();
                if !command.is_empty() {
                    commands.push(command.to_owned());
                }
                current.clear();
            }
            '%' | '\r' | '\n' => {}
            _ => current.push(character),
        }
    }
    let command = current.trim();
    if !command.is_empty() {
        commands.push(command.to_owned());
    }
    commands
}

fn parse_fields(token: &str) -> Vec<(char, String)> {
    let bytes = token.as_bytes();
    let mut fields = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_alphabetic() {
            let letter = bytes[index] as char;
            index += 1;
            let start = index;
            if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
                index += 1;
            }
            while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
                index += 1;
            }
            if index > start {
                fields.push((letter, token[start..index].to_owned()));
            }
        } else {
            index += 1;
        }
    }
    fields
}

fn field_values(fields: &[(char, String)], letter: char) -> impl Iterator<Item = &str> {
    fields
        .iter()
        .filter_map(move |(name, value)| (*name == letter).then_some(value.as_str()))
}

fn last_field(fields: &[(char, String)], letter: char) -> Option<&str> {
    field_values(fields, letter).last()
}

fn format_pair(token: &str, axis: char) -> Option<(usize, usize)> {
    let index = token.find(axis)? + 1;
    let bytes = token.as_bytes();
    let integer = (*bytes.get(index)? as char).to_digit(10)? as usize;
    let decimal = (*bytes.get(index + 1)? as char).to_digit(10)? as usize;
    Some((integer, decimal))
}

fn parse_number(value: &str, integer: usize, decimal: usize, trailing: bool) -> f64 {
    if value.contains('.') {
        return value.parse::<f64>().unwrap_or(0.0);
    }
    let negative = value.starts_with('-');
    let unsigned = value.trim_start_matches(['+', '-']);
    let mut number = unsigned.parse::<i64>().unwrap_or(0) as f64;
    if trailing {
        let omitted = integer
            .saturating_add(decimal)
            .saturating_sub(unsigned.len());
        number *= 10_f64.powi(omitted as i32);
    }
    number /= 10_f64.powi(decimal as i32);
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
    fn parses_standard_rs274x_lines_and_flashes() {
        let source = "%FSLAX24Y24*%%MOMM*%%ADD10C,0.200*%D10*X000000Y000000D02*X100000Y000000D01*X100000Y050000D03*M02*";
        let layer = parse_gerber(source);
        assert_eq!(layer.primitives.len(), 2);
        assert_eq!(layer.pad_count, 1);
        assert_eq!(layer.min_track_width_mm, Some(0.2));
        assert!((layer.bounds.max_x - 10.1).abs() < 0.0001);
    }

    #[test]
    fn parses_trailing_zero_coordinates() {
        assert_eq!(parse_number("12", 2, 4, true), 12.0);
        assert_eq!(parse_number("12", 2, 4, false), 0.0012);
    }

    #[test]
    fn approximates_aperture_macros_from_primitive_bounds() {
        let source =
            "%FSLAX24Y24*%%MOMM*%%AMBOX*21,1,1.2,0.8,0,0,0*%%ADD10BOX*%D10*X000000Y000000D03*M02*";
        let layer = parse_gerber(source);
        assert_eq!(layer.primitives.len(), 1);
        assert_eq!(layer.pad_count, 1);
        assert!((layer.bounds.max_x - layer.bounds.min_x - 1.2).abs() < 0.0001);
        assert!((layer.bounds.max_y - layer.bounds.min_y - 0.8).abs() < 0.0001);
    }
}
