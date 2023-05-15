use crate::OcclusionStatus::Occluded;
use box_intersect_ze::boxes::BBox;
use box_intersect_ze::set::BBoxSet;
use box_intersect_ze::*;
use std::ops::RangeFrom;

const EPS: f32 = 0.00;
const NOWHERE: f32 = f32::MAX;

pub type BOX = boxes::Box2Df32;


#[pyclass]
#[derive(Clone)]
pub struct PyOcclusionBuffer {
    occl_buf:OcclusionBuffer,
}

#[pymethods]
impl PyOcclusionBuffer {
    #[new]
    pub fn new(bot:[f32;2],top:[f32;2]) -> Self {
        Self {
            occl_buf:OcclusionBuffer::new(BOX::new(bot,top))
        }
    }
    /// check if a new box intersects free space
    pub fn check_a_box(&mut self, new: BOX) -> OcclusionStatus {

    }

    /// Adds box that was last passed into check_a_box
    pub fn add_last_box(&mut self) {

    }

    /// add multiple boxes into zbuffer while cutting space for each one
    pub fn add_box_set(&mut self, boxes: Vec<BOX>) {

    }
}
pub enum OcclusionStatus {
    Occluded,
    PartiallyVisible,
}
#[derive(Clone)]
pub struct OcclusionBuffer {
    pub free_space: set::BBoxSet<BOX, usize>,
    //this is silly but library wants this to store the box for inters check
    new_box: set::BBoxSet<BOX, usize>,
    box_idx_alloc: std::ops::RangeFrom<usize>,
    occlusion_status: Vec<(usize, usize)>,
    // boxes of free space that are not in use at the moment (i.e. holes in free_space array)
    dead_boxes: Vec<usize>,
}

#[allow(dead_code)]
impl OcclusionBuffer {
    pub fn new(freespace: BOX) -> Self {
        OcclusionBuffer {
            free_space: {
                let mut s = set::BBoxSet::with_capacity(256);
                s.push(0, freespace);
                s
            },
            new_box: set::BBoxSet::new(),
            box_idx_alloc: 1..,
            occlusion_status: Vec::with_capacity(128),
            dead_boxes: vec![],
        }
    }

    /// check if a new box intersects free space
    pub fn check_a_box(&mut self, new: BOX) -> OcclusionStatus {
        self.new_box.clear();
        self.occlusion_status.clear();
        self.new_box.push(usize::MAX - 1, new);
        intersect_brute_force(&self.free_space, &self.new_box, &mut self.occlusion_status);

        if self.occlusion_status.is_empty() {
            OcclusionStatus::Occluded
        } else {
            OcclusionStatus::PartiallyVisible
        }
    }

    /// Adds box that was last passed into check_a_box
    pub fn add_last_box(&mut self) {
        //take stuff from self.new_box
        assert!(!self.occlusion_status.is_empty());
        debug_assert!(!self.new_box.empty());

        let newbox = self.new_box.boxes[0].0;
        // break up free space to accommodate new box
        cut_space(
            &mut self.free_space,
            &mut self.dead_boxes,
            &self.occlusion_status,
            newbox,
            &mut self.box_idx_alloc,
        );
    }

    /// add multiple boxes into zbuffer while cutting space for each one
    pub fn add_box_set(&mut self, boxes: Vec<BOX>) {
        for b in boxes {
            match self.check_a_box(b) {
                OcclusionStatus::Occluded => {
                    continue;
                }
                OcclusionStatus::PartiallyVisible => self.add_last_box(),
            }
        }
    }
}
/** Checks if new box intersects free space.
Returns vec of indices of free space regions intersected.
 */
fn intersection_check(free_space: &set::BBoxSet<BOX, usize>, new: BOX) -> Vec<(usize, usize)> {
    // create set for comparing two sets intersection
    let mut new_set = set::BBoxSet::new();
    new_set.push(usize::MAX - 1, new);
    println!(
        "inters check free space : {:?}, new: {:?}",
        free_space.boxes, new
    );

    //create set which will collect the indices of the boxes from the old
    //set of boxes, which were intersected
    let mut result = Vec::new();
    //find intersections and put it to the result vector
    //TODO: once bug https://github.com/derivator/box_intersect_ze/issues/2 is fixed return to scan
    //intersect_scan(free_space, &new_set, &mut result);
    //intersect_brute_force(free_space, &new_set, &mut result);
    intersect_brute_force_flex(free_space, &new_set, AnswerFormat::Index(&mut result));
    // Collect only indices from the old set

    println!("List of intersections{:?}", result);
    //result.iter().map(|e| e.0).collect()
    result
}

/** Given vector of free space boxes and vec of indices of free space regions intersected by box new,
breaks up boxes in free space until everything is correct again.
Returns number of new boxes added to free space set
 */

fn two_vertex_intersection_subdivision(
    free: &mut BOX,
    new_min: (f32, f32),
    new_max: (f32, f32),
    free_min: (f32, f32),
    free_max: (f32, f32),
    new_verts_in_free: [bool; 4],
    reverse: bool,
) -> Option<[BOX; 2]> {
    // Two vertex intersection
    //     free.contains_in(0, new_min.0) && free.contains_in(1, new_min.1), left lower corner
    //     free.contains_in(0, new_min.0) && free.contains_in(1, new_max.1), left upper corner
    //     free.contains_in(0, new_max.0) && free.contains_in(1, new_max.1), right upper corner
    //     free.contains_in(0, new_max.0) && free.contains_in(1, new_min.1), right lower corner

    match new_verts_in_free {
        [true, true, _, _] => {
            if reverse {
                *free = BOX::new(
                    [free_max.0 + EPS, new_min.1 + EPS],
                    [new_max.0 - EPS, new_max.1 - EPS],
                );
                None
            } else {
                *free = BOX::new(
                    [free_min.0 + EPS, free_min.1 + EPS],
                    [new_min.0 - EPS, free_max.1 - EPS],
                );
                // right upper corner 1

                Some([
                    BOX::new(
                        [new_min.0 + EPS, new_max.1 + EPS],
                        [free_max.0 - EPS, free_max.1 - EPS],
                    ),
                    //right lower corner 2
                    BOX::new(
                        [new_min.0 + EPS, free_min.1 + EPS],
                        [free_max.0 - EPS, new_min.1 - EPS],
                    ),
                ])
            }
        }

        [_, true, true, _] => {
            if reverse {
                *free = BOX::new(
                    [new_min.0 + EPS, new_min.1 + EPS],
                    [new_max.0 - EPS, free_min.1 - EPS],
                );
                return None;
            }
            // left
            *free = BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_min.0 - EPS, free_max.1 - EPS],
            );
            // middle

            Some([
                BOX::new(
                    [new_min.0 + EPS, new_max.1 + EPS],
                    [new_max.0 - EPS, free_max.1 - EPS],
                ),
                // right
                BOX::new(
                    [new_max.0 + EPS, free_min.1 + EPS],
                    [free_max.0 - EPS, free_max.1 - EPS],
                ),
            ])
        }

        // new overlaps from the right
        [_, _, true, true] => {
            if reverse {
                *free = BOX::new(
                    [new_min.0 + EPS, new_min.1 + EPS],
                    [free_min.0 - EPS, new_max.1 - EPS],
                );
                return None;
            }

            *free = BOX::new(
                [free_min.0 + EPS, new_max.1 + EPS],
                [new_max.0 - EPS, free_max.1 - EPS],
            );

            Some([
                BOX::new(
                    [new_max.0 + EPS, free_min.1 + EPS],
                    [free_max.0 - EPS, free_max.1 - EPS],
                ),
                BOX::new(
                    [free_min.0 + EPS, free_min.1 + EPS],
                    [new_max.0 - EPS, new_min.1 - EPS],
                ),
            ])
            ////
        }
        // new overlaps from down
        [true, _, _, true] => {
            if reverse {
                *free = BOX::new(
                    [new_min.0 + EPS, free_max.1 + EPS],
                    [new_max.0 - EPS, new_max.1 - EPS],
                );
                return None;
            }

            // left
            *free = BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_min.0 - EPS, free_max.1 - EPS],
            );
            // middle

            Some([
                BOX::new(
                    [new_min.0 + EPS, free_min.1 + EPS],
                    [new_max.0 - EPS, new_min.1 - EPS],
                ),
                // right
                BOX::new(
                    [new_max.0 + EPS, free_min.1 + EPS],
                    [free_max.0 - EPS, free_max.1 - EPS],
                ),
            ])
            ////
        }
        _ => {
            unreachable!()
        }
    }
}

/// Checks which vertices are contained in given AABB.
/// If none are contained, array of false is returned.
/// ordering of returned values is [left lower ,left upper, right upper ,right lower]
fn identify_intersection_case(
    aabb: BOX,
    min_vertex: (f32, f32),
    max_vertex: (f32, f32),
) -> [bool; 4] {
    [
        aabb.contains_in(0, min_vertex.0) && aabb.contains_in(1, min_vertex.1),
        aabb.contains_in(0, min_vertex.0) && aabb.contains_in(1, max_vertex.1),
        aabb.contains_in(0, max_vertex.0) && aabb.contains_in(1, max_vertex.1),
        aabb.contains_in(0, max_vertex.0) && aabb.contains_in(1, min_vertex.1),
    ]
}

///
fn cut_space(
    mut free_space: &mut set::BBoxSet<BOX, usize>,
    mut tokill: &mut Vec<usize>,
    intersected: &[(usize, usize)],
    new: BOX,
    start_idx: &mut std::ops::RangeFrom<usize>,
) {
    fn maybe_push(tokill: &mut Vec<usize>, freesp: &mut BBoxSet<BOX, usize>, b: BOX, i: usize) {
        match tokill.pop() {
            Some(v) => freesp.boxes[v] = (b, i),
            None => {
                freesp.push(i, b);
            }
        }
    }

    for &(i, _) in intersected.iter() {
        let (free, _fsp_index) = free_space.boxes.get_mut(i).unwrap();

        let free_min = (free.lo(0), free.lo(1));
        let free_max = (free.hi(0), free.hi(1));

        let new_min = (new.lo(0), new.lo(1));
        let new_max = (new.hi(0), new.hi(1));

        let new_verts_in_free = identify_intersection_case(*free, new_min, new_max);
        let free_verts_in_new = identify_intersection_case(new, free_min, free_max);

        let new_in_free_count = new_verts_in_free.iter().map(|&e| e as u8).sum();
        let free_in_new_count = free_verts_in_new.iter().map(|&e| e as u8).sum();

        match (new_in_free_count, free_in_new_count) {
            (0, 4) => {
                // new entirely contains free,  kill free completely
                *free = BOX::new([NOWHERE, NOWHERE], [NOWHERE, NOWHERE]);
                tokill.push(i)
            }
            (4, 0) => {
                //free entirely contains new, break free into 4 segments
                // left
                *free = BOX::new(
                    [free_min.0 + EPS, free_min.1 + EPS],
                    [new_min.0 - EPS, free_max.1 - EPS],
                );
                // bottom
                maybe_push(
                    &mut tokill,
                    &mut free_space,
                    BOX::new(
                        [new_min.0 + EPS, new_max.1 + EPS],
                        [new_max.0 - EPS, free_max.1 - EPS],
                    ),
                    start_idx.next().unwrap(),
                );
                //top
                free_space.push(
                    start_idx.next().unwrap(),
                    BOX::new(
                        [new_min.0 + EPS, free_min.1 + EPS],
                        [new_max.0 - EPS, new_min.1 - EPS],
                    ),
                );
                // right
                free_space.push(
                    start_idx.next().unwrap(),
                    BOX::new(
                        [new_max.0 + EPS, free_min.1 + EPS],
                        [free_max.0 - EPS, free_max.1 - EPS],
                    ),
                );
            }
            (1, 1) => {
                let rotation = new_verts_in_free.iter().position(|&e| e).unwrap();
                let b =
                    one_vertex_intersection(free, free_min, free_max, new_min, new_max, rotation);
                println!(" BOX Ppushed to free space{:?}", b);
                free_space.push(start_idx.next().unwrap(), b);
            }

            (2, 0) => {
                let space = two_vertex_intersection_subdivision(
                    free,
                    new_min,
                    new_max,
                    free_min,
                    free_max,
                    new_verts_in_free,
                    false,
                );
                match space {
                    None => {}
                    Some(b) => {
                        for i in b {
                            free_space.push(start_idx.next().unwrap(), i);
                        }
                    }
                }
            }

            (0, 2) => {
                println!("DID WE GET HERE ??");
                let space = two_vertex_intersection_subdivision(
                    free,
                    free_min,
                    free_max,
                    new_min,
                    new_max,
                    free_verts_in_new,
                    true,
                );
                match space {
                    None => {}
                    Some(b) => {
                        for i in b {
                            free_space.push(start_idx.next().unwrap(), i);
                        }
                    }
                }
            }
            (3, _) => {
                unreachable!()
            }
            (_, 3) => {
                unreachable!()
            }

            (0, 0) => {
                let (vertical_x, vertical_y) = (
                    new_min.0 > free_min.0 && new_max.0 < free_max.0,
                    new_min.1 < free_min.1 && new_max.1 > free_max.1,
                );

                let (horizontal_x, horizontal_y) = (
                    new_min.0 < free_min.0 && new_max.0 > free_max.0,
                    new_min.1 > free_min.1 && new_max.1 < free_max.1,
                );

                match (vertical_x & vertical_y, horizontal_x & horizontal_y) {
                    (true, false) => {
                        *free = BOX::new(
                            [free_min.0 + EPS, free_min.1 + EPS],
                            [new_min.0 - EPS, free_max.1 - EPS],
                        );
                        // bottom
                        free_space.push(
                            start_idx.next().unwrap(),
                            BOX::new(
                                [new_max.0 + EPS, free_min.1 + EPS],
                                [free_max.0 - EPS, free_max.1 - EPS],
                            ),
                        );
                    }
                    (false, true) => {
                        *free = BOX::new(
                            [free_min.0 + EPS, new_max.1 + EPS],
                            [free_max.0 - EPS, free_max.1 - EPS],
                        );
                        // bottom
                        free_space.push(
                            start_idx.next().unwrap(),
                            BOX::new(
                                [free_min.0 + EPS, free_min.1 + EPS],
                                [free_max.0 - EPS, new_min.1 - EPS],
                            ),
                        );
                    }

                    (_, _) => {}
                }
            }

            (_, _) => {
                unreachable!()
            }
        }
    }

    // start_idx.next();
    // free_space.boxes.remove(0);
    free_space.sort();
}

fn one_vertex_intersection(
    free: &mut BOX,
    free_min: (f32, f32),
    free_max: (f32, f32),
    new_min: (f32, f32),
    new_max: (f32, f32),
    rotation: usize,
) -> BOX {
    // One vertex intersection

    match rotation {
        0 => {
            //Right upper corner intersection

            // Left
            *free = BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_min.0 - EPS, free_max.1 - EPS],
            );

            // Right

            BOX::new(
                [new_min.0 + EPS, free_min.1 + EPS],
                [free_max.0 - EPS, new_min.1 - EPS],
            )
        }

        1 => {
            //Right lower corner

            // Left lower corner
            *free = BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_min.0 - EPS, free_max.1 - EPS],
            );

            // Left upper corner box

            BOX::new(
                [new_min.0 + EPS, new_max.1 + EPS],
                [free_max.0 - EPS, free_max.1 - EPS],
            )
        }
        2 => {
            //////////

            // Left lower corner intersection
            //left

            *free = BOX::new(
                [free_min.0 + EPS, new_max.1 + EPS],
                [new_max.0 - EPS, free_max.1 - EPS],
            );

            //right
            BOX::new(
                [new_max.0 + EPS, free_min.1 + EPS],
                [free_max.0 - EPS, free_max.1 - EPS],
            )
            //Checked
            ////////////
        }
        3 => {
            ////

            // Left upper corner intersection

            // left
            *free = BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_max.0 - EPS, new_min.1 - EPS],
            );

            // right

            BOX::new(
                [new_max.0 + EPS, free_min.1 + EPS],
                [free_max.0 - EPS, free_max.1 - EPS],
            )
            /////
        }
        _ => {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use box_intersect_ze::boxes::BBox;

    use plotters::prelude::*;
    use plotters::style::full_palette::BLUE_50;
    use stdext::function_name;

    #[test]
    fn occlusion_buffer() {
        /*
        let mut free_space = set::BBoxSet::<BOX, usize>::new();
        let f_s = BOX::new([zb_size.0.0, zb_size.0.1], [zb_size.1.0, zb_size.1.1]);
        free_space.push(0, f_s);

        let mut buffer = OcclusionBuffer::new();

        buffer.free_space = free_space;
        buffer.add_box_set(set);

        for b in buffer.new_box{

            let status= buffer.check_a_box(b);
            if status == Occluded {continue}
            else{
                buffer.add_last_box()
            }

        }*/
    }

    fn test_inner_multiple(free: Vec<BOX>, new: BOX, name: String, num_inters: usize) {
        let mut free_space = set::BBoxSet::<BOX, usize>::new();
        let mut index_alloc = 1..;
        for v in free {
            free_space.push(index_alloc.next().unwrap(), v);
        }
        free_space.sort();

        plotboxes(&free_space, new, &(name.clone() + "__before.svg"));

        let inters = intersection_check(&free_space, new);
        assert_eq!(inters.len(), num_inters);

        //println!("inters array is {:?}", inters);
        //println!("free space before {:?}", free_space.boxes);
        let mut tokill = vec![];
        cut_space(&mut free_space, &mut tokill, &inters, new, &mut index_alloc);
        free_space.sort();
        //println!("free space after {:?}", free_space.boxes);
        {
            let mut res = vec![];
            intersect_scan(&free_space, &free_space, &mut res);
            assert_eq!(
                res.len(),
                0,
                "free space should not have self-intersections"
            );
            let inters = intersection_check(&free_space, new);
            assert_eq!(
                inters.len(),
                0,
                "free space should not intersect new after cut is done"
            );
        }
        println!(" Free space{:?} NEW: {:?}", free_space.boxes, new);
        plotboxes(&free_space, new, &(name + "_after.svg"));
    }
    ///
    /// * `free`
    /// * `new`
    fn test_inner(free: BOX, new: BOX, name: String, num_inters: usize) {
        test_inner_multiple(vec![free], new, name, num_inters);
    }

    #[test]
    pub fn test_multiple_free_space() {
        let free = vec![BOX::new([0., 0.], [0.5, 1.]), BOX::new([0.5, 0.], [1., 1.])];

        let new = BOX::new([0.2, 0.2], [0.7, 0.7]); //inside the empty
        test_inner_multiple(free, new, "multiple".to_string(), 2);
    }

    #[test]
    pub fn test_full_overlap() {
        let free = BOX::new([0., 0.], [1., 1.]); // base
        let new = BOX::new([0.2, 0.2], [0.7, 0.7]); //inside the empty
        test_inner(
            free,
            new,
            format!("test_plots/{} {}", &function_name!(), 1),
            1,
        );
    }

    #[test]
    pub fn test_corner_overlap() {
        let cases = vec![
            BOX::new([0.7, -0.3], [1.3, 0.3]),
            BOX::new([0.7, 0.7], [1.3, 1.3]),
            BOX::new([-0.3, 0.7], [0.3, 1.3]),
            BOX::new([-0.3, -0.3], [0.3, 0.3]),
        ];
        let free = BOX::new([0., 0.], [1., 1.]); // base

        for (i, a) in cases.iter().enumerate() {
            test_inner(
                free,
                *a,
                format!("test_corner_overlap/{} {}", &function_name!(), i),
                1,
            );
        }
    }

    #[test]
    pub fn test_2vertex_overlap() {
        let cases = vec![
            BOX::new([-0.2, 0.2], [0.7, 0.7]),
            BOX::new([0.4, 0.7], [0.8, 1.3]),
            BOX::new([0.7, 0.3], [1.3, 0.7]),
            BOX::new([0.3, -0.3], [0.6, 0.3]),
        ];
        let free = BOX::new([0., 0.], [1., 1.]); // base
        for (i, a) in cases.iter().enumerate() {
            test_inner(
                free,
                *a,
                format!("two_vertex_overlap/{} {}", &function_name!(), i),
                1,
            );
        }
    }

    #[test]
    pub fn test_no_vertex_overlap() {
        let cases = vec![
            BOX::new([-0.3, 0.2], [1.3, 0.7]),
            BOX::new([0.3, -0.2], [0.6, 1.3]),
        ];

        let free = BOX::new([0., 0.], [1., 1.]); // base
        for (i, a) in cases.iter().enumerate() {
            test_inner(
                free,
                *a,
                format!("vert_horizont_div/{} {}", &function_name!(), i),
                1,
            );
        }
    }

    #[test]
    pub fn height_width_overlap() {
        let cases = vec![
            BOX::new([0.7, -0.3], [1.3, 1.3]),
            BOX::new([-0.2, 0.7], [1.3, 1.3]),
            BOX::new([-0.2, -0.2], [0.2, 1.3]),
            BOX::new([-0.2, -0.2], [1.3, 0.3]),
        ];

        let free = BOX::new([0., 0.], [1., 1.]); // base

        for (i, a) in cases.iter().enumerate() {
            test_inner(
                free,
                *a,
                format!("{} {}", &better_name(function_name!()), i),
                1,
            );
        }
    }
    #[test]
    pub fn free_in_new() {
        let free = BOX::new([0., 0.], [1., 1.]); // base
        let new = BOX::new([-0.3, -0.3], [1.3, 1.3]);
        test_inner(
            free,
            new,
            format!("{} {}", &better_name(function_name!()), 1),
            1,
        );
    }

    fn plotboxes(free_space: &set::BBoxSet<BOX, usize>, new: BOX, name: &str) {
        let mut backend = SVGBackend::new(name, (MAX_PIX as u32, MAX_PIX as u32));
        let style = {
            let f = SVGBackend::new("/dev/null", (150, 150)).into_drawing_area();
            ("sans-serif", 15.0, &BLUE).into_text_style(&f)
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
        if let Some((lo, hi)) = project_coords(new) {
            backend.draw_rect(lo, hi, &RED, false).unwrap();
            backend.present().unwrap();
        }
    }

    fn better_name(s: &str) -> String {
        s.to_string().split("::").last().unwrap().to_string()
    }
    const MAX_PIX: i32 = 256;
    fn project_coords(b: BOX) -> Option<((i32, i32), (i32, i32))> {
        if b.lo(0) == NOWHERE {
            return None;
        }

        let res = (
            ((b.lo(0) * 100.) as i32 + 100, (b.lo(1) * 100.) as i32 + 100),
            ((b.hi(0) * 100.) as i32 + 100, (b.hi(1) * 100.) as i32 + 100),
        );
        assert!(res.0 .0 > 0);
        assert!(res.0 .1 < MAX_PIX);
        assert!(res.1 .0 > 0);
        assert!(res.1 .1 < MAX_PIX);
        Some(res)
    }
}
