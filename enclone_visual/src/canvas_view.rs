// Copyright (c) 2021 10X Genomics, Inc. All rights reserved.

use iced::{
    canvas::{self, Canvas, Cursor, Frame, Geometry, Path, Stroke, Text},
    Color, Element, HorizontalAlignment, Length, Rectangle, Size, VerticalAlignment,
};
use iced_native::{Font, Point, Vector};
use string_utils::*;
use tables::*;

const DEJAVU: Font = Font::External {
    name: "DEJAVU",
    bytes: include_bytes!("../../fonts/DejaVuLGCSansMono.ttf"),
};

const LIBERATION_SANS: Font = Font::External {
    name: "LIBERATION_SANS",
    bytes: include_bytes!("../../fonts/LiberationSans-Regular.ttf"),
};

#[derive(Default)]
pub struct State {
    pub geometry_value: Option<Vec<crate::geometry::Geometry>>,
}

pub struct CanvasView {
    pub state: State,
}

#[derive(Debug, Clone)]
pub enum Message {}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            state: State::default(),
        }
    }
}

impl CanvasView {
    pub fn view<'a>(&'a mut self) -> Element<'a, Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn to_color(c: &crate::geometry::Color) -> Color {
    Color {
        r: c.r as f32 / 255.0,
        g: c.g as f32 / 255.0,
        b: c.b as f32 / 255.0,
        a: c.t as f32 / 255.0,
    }
}

impl<'a> canvas::Program<Message> for CanvasView {
    fn draw(&self, bounds: Rectangle, cursor: Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(bounds.size());
        if self.state.geometry_value.is_some() {
            // for now not scaling stroke width, not sure what is optimal
            // scaling seems to be needed only because .height(SVG_WIDTH) doesn't work on a canvas
            // should file bug
            const MAX_HEIGHT: f32 = 400.0;
            let mut height = 0.0 as f32;
            let g = self.state.geometry_value.as_ref().unwrap();
            for i in 0..g.len() {
                match &g[i] {
                    crate::geometry::Geometry::Text(o) => {
                        height = height.max(o.p.y);
                    }
                    crate::geometry::Geometry::Rectangle(rect) => {
                        height = height.max(rect.p.y + rect.height);
                    }
                    crate::geometry::Geometry::PolySegment(segs) => {
                        for i in 0..segs.p.len() - 1 {
                            height = height.max(segs.p[i].y);
                        }
                    }
                    crate::geometry::Geometry::Segment(seg) => {
                        height = height.max(seg.p1.y);
                        height = height.max(seg.p2.y);
                    }
                    crate::geometry::Geometry::CircleWithTooltip(circ) => {
                        height = height.max(circ.p.y + circ.r);
                    }
                    crate::geometry::Geometry::Circle(circ) => {
                        height = height.max(circ.p.y + circ.r);
                    }
                };
            }
            let mut scale = 1.0;
            if height > MAX_HEIGHT {
                scale = MAX_HEIGHT / height;
            }
            for i in 0..g.len() {
                match &g[i] {
                    crate::geometry::Geometry::Text(o) => {
                        // rotate not implemented because not a feature yet in iced
                        let x = Text {
                            content: o.t.clone(),
                            size: o.font_size * scale,
                            color: to_color(&o.c),
                            position: Point { x: o.p.x * scale, y: o.p.y * scale },
                            font: match o.font.as_str() {
                                "DejaVuSansMono" => DEJAVU,
                                "Arial" => LIBERATION_SANS,
                                _ => LIBERATION_SANS,
                            },
                            vertical_alignment: VerticalAlignment::Bottom,
                            horizontal_alignment: match o.halign {
                                crate::geometry::HorizontalAlignment::Left => {
                                    HorizontalAlignment::Left
                                }
                                crate::geometry::HorizontalAlignment::Center => {
                                    HorizontalAlignment::Center
                                }
                                crate::geometry::HorizontalAlignment::Right => {
                                    HorizontalAlignment::Right
                                }
                            },
                        };
                        frame.fill_text(x);
                    }
                    crate::geometry::Geometry::Rectangle(rect) => {
                        let r = Path::rectangle(
                            Point {
                                x: rect.p.x * scale,
                                y: rect.p.y * scale,
                            },
                            Size::new(rect.width * scale, rect.height * scale),
                        );
                        frame.fill(&r, to_color(&rect.fill_color));
                        let c = to_color(&rect.stroke_color);
                        frame.stroke(
                            &r,
                            Stroke::default()
                                .with_color(c)
                                .with_width(rect.stroke_width),
                        );
                    }
                    crate::geometry::Geometry::PolySegment(segs) => {
                        for i in 0..segs.p.len() - 1 {
                            let p = Path::line(
                                Point {
                                    x: segs.p[i].x * scale,
                                    y: segs.p[i].y * scale,
                                },
                                Point {
                                    x: segs.p[i + 1].x * scale,
                                    y: segs.p[i + 1].y * scale,
                                },
                            );
                            let c = to_color(&segs.c);
                            frame.stroke(&p, Stroke::default().with_color(c).with_width(segs.w));
                        }
                    }
                    crate::geometry::Geometry::Segment(seg) => {
                        let p = Path::line(
                            Point {
                                x: seg.p1.x * scale,
                                y: seg.p1.y * scale,
                            },
                            Point {
                                x: seg.p2.x * scale,
                                y: seg.p2.y * scale,
                            },
                        );
                        let c = to_color(&seg.c);
                        frame.stroke(&p, Stroke::default().with_color(c).with_width(seg.w));
                    }
                    crate::geometry::Geometry::CircleWithTooltip(circ) => {
                        let circle = Path::circle(
                            Point {
                                x: circ.p.x * scale,
                                y: circ.p.y * scale,
                            },
                            circ.r * scale,
                        );
                        let c = &circ.c;
                        frame.fill(&circle, to_color(c));
                    }
                    crate::geometry::Geometry::Circle(circ) => {
                        let circle = Path::circle(
                            Point {
                                x: circ.p.x * scale,
                                y: circ.p.y * scale,
                            },
                            circ.r * scale,
                        );
                        let c = &circ.c;
                        frame.fill(&circle, to_color(c));
                    }
                };
            }
            let pos = cursor.position_in(&bounds);
            if pos.is_some() {
                for i in 0..g.len() {
                    match &g[i] {
                        crate::geometry::Geometry::CircleWithTooltip(circ) => {
                            let xdiff = pos.unwrap().x - circ.p.x * scale;
                            let ydiff = pos.unwrap().y - circ.p.y * scale;
                            let dist = (xdiff * xdiff + ydiff * ydiff).sqrt();
                            if dist <= circ.r {
                                let stext = circ.t.clone();
                                let xs = stext.split(',').collect::<Vec<&str>>();
                                let mut rows = Vec::<Vec<String>>::new();
                                for i in 0..xs.len() {
                                    if i > 0 {
                                        rows.push(vec!["\\hline".to_string(); 2]);
                                    }
                                    let mut row = Vec::<String>::new();
                                    row.push(xs[i].before("=").to_string());
                                    row.push(xs[i].after("=").to_string());
                                    rows.push(row);
                                }
                                let mut log = String::new();
                                print_tabular_vbox(
                                    &mut log,
                                    &rows,
                                    1,
                                    &b"l|r".to_vec(),
                                    false,
                                    true,
                                );
                                frame.translate(Vector { x: 400.0, y: 10.0 });
                                let text = canvas::Text {
                                    content: log,
                                    size: 22.0,
                                    font: DEJAVU,
                                    color: Color::from_rgb(0.5, 0.3, 0.3),
                                    ..canvas::Text::default()
                                };
                                frame.fill_text(text);
                                frame.translate(Vector {
                                    x: -400.0,
                                    y: -10.0,
                                });
                                break;
                            }
                        }
                        _ => {}
                    };
                }
            }
        }
        vec![frame.into_geometry()]
    }
}
