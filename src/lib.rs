use std::f32::MIN;
use box_intersect_ze::*;
use box_intersect_ze::boxes::BBox;

pub type BOX = boxes::Box2Df32;

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
}


impl OcclusionBuffer {
    pub fn new() -> Self {
        OcclusionBuffer {
            free_space: set::BBoxSet::new(),
            new_box: set::BBoxSet::new(),
            box_idx_alloc: 1..,
            occlusion_status: Vec::with_capacity(16),
        }
    }


    pub fn check_a_box(&mut self, new: BOX) -> OcclusionStatus {
        self.new_box.clear();
        self.occlusion_status.clear();
        self.new_box.push(usize::MAX - 1, new);
        box_intersect_ze::intersect_scan(
            &self.free_space,
            &self.new_box,
            &mut self.occlusion_status,
        );

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
        // break up free space to accommodate new box
    }

    fn add_freespace_box(&mut self, b: BOX) {
        self.free_space.push(self.box_idx_alloc.next().unwrap(), b)
    }


}
/** Checks if new box intersects free space.
Returns vec of indices of free space regions intersected.
 */
fn intersection_check(free_space: &set::BBoxSet<BOX, usize>, new: BOX) -> Vec<usize> {
    // create set for comparing two sets intersection
    let mut new_set = set::BBoxSet::new();
    new_set.push(usize::MAX - 1, new);

    //create set which will collect the indices of the boxes from the old
    //set of boxes, which were intersected
    let mut result = Vec::new();
    //find intersections and put it to the result vector
    intersect_scan(free_space, &new_set, &mut result);
    // Collect only indices from the old set
    result.iter().map(|e| e.0).collect()
}

fn covered_area_limits(free_space: &set::BBoxSet<BOX, usize>) -> ((f32, f32), (f32, f32)){

    let mut  max =(f32::MIN, f32::MIN);
    let mut  min =(f32::MAX, f32::MAX);

    for i in &free_space.boxes{
        if i.0.lo(0) < min.0 {
            min.0 = i.0.lo(0)
        }
        if i.0.lo(1) < min.1 {
            min.1 = i.0.lo(1)
        }
        if i.0.hi(0) > max.0{
            max.0 = i.0.hi(0)
        }
        if i.0.hi(1) > max.1{
            max.1 = i.0.hi(1)
        }
    }

    (min, max)
}
/** Given vector of free space boxes and vec of indices of free space regions intersected by box new,
breaks up boxes in free space until everything is correct again.
Returns number of new boxes added to free space set
 */
fn cut_space(
    free_space: &mut set::BBoxSet<BOX, usize>,
    intersected: Vec<usize>,
    new: BOX,
    start_idx: &mut std::ops::RangeFrom<usize>,
) {
    // let mut collection_of_occluded_boxes = vec![];
    // for i in intersected.boxes {
    //     collection_of_occluded_boxes.push(free_space.boxes.get_mut(i).unwrap());
    // }
    // let guaranteed_free_space = covered_area_limits(&collection_of_occluded_boxes);
    let mut tokill=vec![];
    for i in intersected {

        let (free, _fsp_index) = free_space.boxes.get_mut(i).unwrap();

        let free_min = (free.lo(0), free.lo(1));
        let free_max = (free.hi(0), free.hi(1));

        let new_min = (new.lo(0), new.lo(1));
        let new_max = (new.hi(0), new.hi(1));

        let inters_vert;

        let new_verts_in = [
            free.contains_in(0, new_min.0) && free.contains_in(1, new_min.1),//
            free.contains_in(0, new_min.0) && free.contains_in(1, new_max.1),
            free.contains_in(0, new_max.0) && free.contains_in(1, new_max.1),
            free.contains_in(0, new_max.0) && free.contains_in(1, new_min.1)
        ];
        inters_vert = new_verts_in.iter().map(|&e| e as usize).sum();
       /// instead od deleting  boxes from free space  we mark them "empty"
       /// and keep them in case we wold need to create more boxes when subdividing free space
        match inters_vert {
            0 =>{
                // kill b because b is inside filled space
                *free = BOX::new([f32::MAX,f32::MAX], [f32::MAX, f32::MAX]);
            }
            1 =>{
                /// One vertex intersection
                let rotation = new_verts_in.iter().position(|&e| e).unwrap();
                match rotation {
                    0=>{
                        // Left lower corner intersection
                        //todo the index is yet hardcoded make it unique
                        //left upper corner
                        *free = BOX::new([new.lo(0), free_max.1], [free_max.0, new.hi(1)]);

                        //right upper corner
                        free_space.push(start_idx.next().unwrap(), BOX::new([free_max.0, free_max.1],
                                                         [new.hi(0), new.hi(1)]));
                        //right lower corner
                        free_space.push(start_idx.next().unwrap(), BOX::new([free_max.0,free_min.1],
                                                    [new.hi(0), free_max.1]));

                        }
                    1=>{
                        // Left upper corner intersection

                        // left lower corner
                        *free = BOX::new([new.lo(0),new.lo(1)], [free.hi(0), free.lo(1)]);

                        // right upper corner
                        free_space.push(start_idx.next().unwrap(),BOX::new([free.hi(0), free.lo(1)],
                                                   [new.hi(0), new.hi(1)]));
                        // right lower corner
                        free_space.push(start_idx.next().unwrap(), BOX::new([free.hi(0),new.lo(1)],
                                                    [new.hi(0), free.lo(1)]));

                    }
                    2=>{
                        //Right upper corner intersection

                        // Left lower corner box
                        *free = BOX::new([new.lo(0),new.lo(1)], [free.lo(0), free.lo(1)]);

                        //Left upper  corner box
                        free_space.push(start_idx.next().unwrap(),BOX::new([new.lo(0), free.lo(1)], [free.lo(0), new.hi(1)]));
                        // Right lower corner box
                        free_space.push(start_idx.next().unwrap(),BOX::new([free.lo(0),new.lo(1)], [new.lo(0), free.lo(1)]));

                    }
                    3=>{
                        //Right lower corner
                        // Left lower corner
                        *free =  BOX::new([new.lo(0),new.lo(1)], [free.lo(0), free.hi(1)]);

                        // Left upper corner box
                        free_space.push(start_idx.next().unwrap(),BOX::new([new.lo(0), free.hi(1)], [free.lo(0), new.hi(1)]));
                        // Right upper corner
                        free_space.push(start_idx.next().unwrap(),BOX::new([free.lo(0), free.hi(1)], [new.hi(0), new.hi(1)]));


                    }
                    _ => {}
                }
            }
            2 =>{
                // Two vertex intersection


            }
            3 => {
                unreachable!()
            }
            4 => {

            }
            _=>{unreachable!()}
        }
    }

        //amount of vert intersected free space

    for i in tokill{
        free_space.boxes.remove(i);
    }
    // start_idx.next();
    // free_space.boxes.remove(0);
    free_space.sort();
}





#[cfg(test)]
mod tests {
    use crate::*;
    use box_intersect_ze::boxes::BBox;
    use box_intersect_ze::*;
    use plotters::prelude::*;
    use plotters::prelude::full_palette::GREEN_50;
    use plotters::style::full_palette::BLUE_50;
    use stdext::function_name;

    ///
    /// * `free`
    /// * `new`
    fn test_inner(free: BOX, new: BOX, name: String, num_inters: usize) {
        let mut index_alloc = 1..;
        let mut free_space = set::BBoxSet::<BOX, usize>::new();
        free_space.push(index_alloc.next().unwrap(), free);
        plotboxes(&free_space, new, &(name.clone() + "__before.svg"));

        let inters = intersection_check(&free_space, new);
        assert_eq!(inters.len(), num_inters);
        if num_inters == 0{return;}
        cut_space(&mut free_space, inters, new, &mut index_alloc);
        {
            let mut res = vec![];
            intersect_scan(&free_space, &free_space, &mut res);
            assert_eq!(res.len(), 0);
            let inters = intersection_check(&free_space, new);
            assert_eq!(inters.len(), 0);
        }
        plotboxes(&free_space, new, &(name + "_after.svg"));
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
        let mut cases = vec![
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
        let mut cases = vec![
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
        let mut cases = vec![
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
        let mut cases = vec![
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
                format!("HW_overlap/{} {}", &better_name(function_name!()), i),
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
            format!("hole/{} {}", &better_name(function_name!()), 1),
            1,
        );
    }

    fn plotboxes(free_space: &set::BBoxSet<BOX, usize>, new: BOX, name: &str) {
        let mut backend = SVGBackend::new(name, (MAX_PIX as u32, MAX_PIX as u32));
        let style = {
            let F = SVGBackend::new("/dev/null", (150, 150)).into_drawing_area();
            ("sans-serif", 15.0, &BLUE).into_text_style(&F)
        };

        for (b, i) in &free_space.boxes {
            let (lo, hi) = project_coords(*b);
            backend.draw_rect(lo, hi, &BLUE_50, true).unwrap();
            backend.draw_rect(lo, hi, &BLUE, false).unwrap();
            backend
                .draw_text(&*i.to_string(), &style, lo)
                .expect("TODO: panic message")
        }
        let (lo, hi) = project_coords(new);
        backend.draw_rect(lo, hi, &RED, false).unwrap();
        backend.present().unwrap();
    }
    fn better_name(s: &str) -> String {
        s.to_string().split("::").last().unwrap().to_string()
    }
     const MAX_PIX: i32 = 256;
    fn project_coords(b: BOX) -> ((i32, i32), (i32, i32)) {
        let res = (
            ((b.lo(0) * 100.) as i32 + 100, (b.lo(1) * 100.) as i32 + 100),
            ((b.hi(0) * 100.) as i32 + 100, (b.hi(1) * 100.) as i32 + 100),
        );
        assert!(res.0 .0 > 0);
        assert!(res.0 .1 < MAX_PIX);
        assert!(res.1 .0 > 0);
        assert!(res.1 .1 < MAX_PIX);
        res
    }
}
