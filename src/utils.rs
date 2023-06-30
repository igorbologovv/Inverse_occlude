use box_intersect_ze::boxes::BBox;
use plotters::prelude::*;
use plotters::style::full_palette::BLUE_50;

use crate::box_cutting::*;
use box_intersect_ze::*;

const MAX_PIX: i32 = 256;
const FONT_SIZE: f32 = 8.0;
pub fn plotboxes(
    free_space: &set::BBoxSet<BOX, usize>,
    new: &set::BBoxSet<BOX, usize>,
    name: &str,
) {
    let name = match name.ends_with(".svg") {
        true => name.to_owned(),
        false => name.to_owned() + ".svg",
    };
    let mut backend = SVGBackend::new(&name, (MAX_PIX as u32, MAX_PIX as u32));
    let style = {
        let f = SVGBackend::new("/dev/null", (150, 150)).into_drawing_area();
        ("sans-serif", FONT_SIZE, &BLUE).into_text_style(&f)
    };

    for (b, i) in &free_space.boxes {
        if let Some((lo, hi)) = project_coords(*b) {
            backend.draw_rect(lo, hi, &BLUE_50, true).unwrap();
            backend.draw_rect(lo, hi, &BLUE, false).unwrap();
            backend
                .draw_text(&*i.to_string(), &style, lo)
                .expect("TODO: panic message")
        }
    }
    let style = {
        let f = SVGBackend::new("/dev/null", (150, 150)).into_drawing_area();
        ("sans-serif", FONT_SIZE, &RED).into_text_style(&f)
    };
    for (b, i) in &new.boxes {
        if let Some((lo, hi)) = project_coords(*b) {
            backend.draw_rect(lo, hi, &RED, false).unwrap();
            backend
                .draw_text(&*i.to_string(), &style, lo)
                .expect("TODO: panic message")
        }
    }
    backend.present().unwrap();
}

fn project_coords(b: BOX) -> Option<((i32, i32), (i32, i32))> {
    if b.lo(0) == NOWHERE {
        return None;
    }
    let bias = MAX_PIX / 2;
    let res = (
        (
            (b.lo(0) * 100.) as i32 + bias,
            (-b.hi(1) * 100.) as i32 + bias,
        ),
        (
            (b.hi(0) * 100.) as i32 + bias,
            (-b.lo(1) * 100.) as i32 + bias,
        ),
    );

    Some(res)
}
