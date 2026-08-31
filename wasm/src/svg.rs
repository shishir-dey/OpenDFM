use std::f64::consts::PI;

use crate::model::{ApertureShape, ParsedLayer, Point, Primitive};

pub(crate) fn render_svg_fragment(layer: &ParsedLayer, color: &str) -> String {
    let mut svg = String::new();

    for primitive in &layer.primitives {
        match primitive {
            Primitive::Line { start, end, width } => {
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",
                    number(start.x), number(-start.y), number(end.x), number(-end.y), color, number(*width)
                ));
            }
            Primitive::Arc {
                start,
                end,
                center,
                width,
                clockwise,
            } => {
                let radius = distance(*start, *center);
                if radius <= f64::EPSILON {
                    continue;
                }
                let start_angle = (start.y - center.y).atan2(start.x - center.x);
                let end_angle = (end.y - center.y).atan2(end.x - center.x);
                let delta = if *clockwise {
                    (start_angle - end_angle).rem_euclid(2.0 * PI)
                } else {
                    (end_angle - start_angle).rem_euclid(2.0 * PI)
                };
                let large_arc = usize::from(delta > PI);
                let sweep = usize::from(!*clockwise);
                svg.push_str(&format!(
                    "<path d=\"M {} {} A {} {} 0 {} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",
                    number(start.x), number(-start.y), number(radius), number(radius), large_arc, sweep,
                    number(end.x), number(-end.y), color, number(*width)
                ));
            }
            Primitive::Flash { at, aperture } => match &aperture.shape {
                ApertureShape::Circle => {
                    svg.push_str(&format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
                        number(at.x),
                        number(-at.y),
                        number(aperture.width * 0.5),
                        color
                    ));
                }
                ApertureShape::Rectangle => {
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                        number(at.x - aperture.width * 0.5),
                        number(-at.y - aperture.height * 0.5),
                        number(aperture.width),
                        number(aperture.height),
                        color
                    ));
                }
                ApertureShape::Obround => {
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"/>",
                        number(at.x - aperture.width * 0.5), number(-at.y - aperture.height * 0.5),
                        number(aperture.width), number(aperture.height),
                        number(aperture.width.min(aperture.height) * 0.5), color
                    ));
                }
                ApertureShape::Polygon {
                    vertices,
                    rotation_deg,
                } => {
                    let count = (*vertices).max(3);
                    let rotation = rotation_deg.to_radians();
                    let radius = aperture.width * 0.5;
                    let points = (0..count)
                        .map(|index| {
                            let angle = rotation + 2.0 * PI * index as f64 / count as f64;
                            format!(
                                "{},{}",
                                number(at.x + radius * angle.cos()),
                                number(-(at.y + radius * angle.sin()))
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    svg.push_str(&format!(
                        "<polygon points=\"{}\" fill=\"{}\"/>",
                        points, color
                    ));
                }
            },
            Primitive::Region { points } => {
                if points.len() < 3 {
                    continue;
                }
                let points = points
                    .iter()
                    .map(|point| format!("{},{}", number(point.x), number(-point.y)))
                    .collect::<Vec<_>>()
                    .join(" ");
                svg.push_str(&format!(
                    "<polygon points=\"{}\" fill=\"{}\" fill-rule=\"evenodd\"/>",
                    points, color
                ));
            }
            Primitive::Drill { at, diameter } => {
                svg.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" fill-opacity=\"0.9\"/>",
                    number(at.x),
                    number(-at.y),
                    number(diameter * 0.5),
                    color
                ));
            }
        }
    }

    svg
}

fn distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn number(value: f64) -> String {
    let value = if value.abs() < 0.000_000_5 {
        0.0
    } else {
        value
    };
    let mut formatted = format!("{value:.6}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}
