use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub(crate) enum ApertureShape {
    Circle,
    Rectangle,
    Obround,
    Polygon { vertices: usize, rotation_deg: f64 },
}

#[derive(Debug, Clone)]
pub(crate) struct Aperture {
    pub shape: ApertureShape,
    pub width: f64,
    pub height: f64,
}

impl Default for Aperture {
    fn default() -> Self {
        Self {
            shape: ApertureShape::Circle,
            width: 0.1,
            height: 0.1,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Primitive {
    Line {
        start: Point,
        end: Point,
        width: f64,
    },
    Arc {
        start: Point,
        end: Point,
        center: Point,
        width: f64,
        clockwise: bool,
    },
    Flash {
        at: Point,
        aperture: Aperture,
    },
    Region {
        points: Vec<Point>,
    },
    Drill {
        at: Point,
        diameter: f64,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub empty: bool,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
            empty: true,
        }
    }
}

impl Bounds {
    pub fn include(&mut self, point: Point, margin: f64) {
        let min_x = point.x - margin;
        let min_y = point.y - margin;
        let max_x = point.x + margin;
        let max_y = point.y + margin;

        if self.empty {
            self.min_x = min_x;
            self.min_y = min_y;
            self.max_x = max_x;
            self.max_y = max_y;
            self.empty = false;
        } else {
            self.min_x = self.min_x.min(min_x);
            self.min_y = self.min_y.min(min_y);
            self.max_x = self.max_x.max(max_x);
            self.max_y = self.max_y.max(max_y);
        }
    }

    pub fn width(&self) -> f64 {
        if self.empty {
            0.0
        } else {
            self.max_x - self.min_x
        }
    }

    pub fn height(&self) -> f64 {
        if self.empty {
            0.0
        } else {
            self.max_y - self.min_y
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ParsedLayer {
    pub primitives: Vec<Primitive>,
    pub bounds: Bounds,
    pub warnings: Vec<String>,
    pub pad_count: usize,
    pub hole_count: usize,
    pub min_track_width_mm: Option<f64>,
    pub min_hole_diameter_mm: Option<f64>,
}

impl ParsedLayer {
    pub fn add_primitive(&mut self, primitive: Primitive) {
        match &primitive {
            Primitive::Line { start, end, width } => {
                let margin = *width * 0.5;
                self.bounds.include(*start, margin);
                self.bounds.include(*end, margin);
                if *width > 0.0 {
                    self.min_track_width_mm = Some(
                        self.min_track_width_mm
                            .map_or(*width, |value| value.min(*width)),
                    );
                }
            }
            Primitive::Arc {
                start,
                end,
                center,
                width,
                ..
            } => {
                let radius = ((start.x - center.x).powi(2) + (start.y - center.y).powi(2)).sqrt();
                self.bounds.include(*center, radius + *width * 0.5);
                self.bounds.include(*end, *width * 0.5);
                if *width > 0.0 {
                    self.min_track_width_mm = Some(
                        self.min_track_width_mm
                            .map_or(*width, |value| value.min(*width)),
                    );
                }
            }
            Primitive::Flash { at, aperture } => {
                self.bounds.include(
                    Point {
                        x: at.x - aperture.width * 0.5,
                        y: at.y - aperture.height * 0.5,
                    },
                    0.0,
                );
                self.bounds.include(
                    Point {
                        x: at.x + aperture.width * 0.5,
                        y: at.y + aperture.height * 0.5,
                    },
                    0.0,
                );
                self.pad_count += 1;
            }
            Primitive::Region { points } => {
                for point in points {
                    self.bounds.include(*point, 0.0);
                }
            }
            Primitive::Drill { at, diameter } => {
                self.bounds.include(*at, *diameter * 0.5);
                self.hole_count += 1;
                if *diameter > 0.0 {
                    self.min_hole_diameter_mm = Some(
                        self.min_hole_diameter_mm
                            .map_or(*diameter, |value| value.min(*diameter)),
                    );
                }
            }
        }
        self.primitives.push(primitive);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub is_empty: bool,
}

impl From<Bounds> for LayerBounds {
    fn from(bounds: Bounds) -> Self {
        if bounds.empty {
            return Self {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 0.0,
                max_y: 0.0,
                is_empty: true,
            };
        }

        Self {
            min_x: bounds.min_x,
            min_y: -bounds.max_y,
            max_x: bounds.max_x,
            max_y: -bounds.min_y,
            is_empty: false,
        }
    }
}
