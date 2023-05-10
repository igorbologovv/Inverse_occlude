use box_intersect_ze::boxes::BBox;
use box_intersect_ze::*;



const EPS: f32 = 0.05;

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
#[allow(dead_code)]
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
    println!("inters check free space : {:?}, new: {:?}", free_space.boxes, new) ;
    //create set which will collect the indices of the boxes from the old
    //set of boxes, which were intersected
    let mut result = Vec::new();
    //find intersections and put it to the result vector
    intersect_scan(free_space, &new_set, &mut result);
    // Collect only indices from the old set

    println!("List of intersections{:?}", result );
    result.iter().map(|e| e.1).collect()
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
    new_verts_in_free: [bool;4]
    )->[BOX;2]{
     // Two vertex intersection
                match new_verts_in_free {
                     [true,true,_,_] => {
                    //left upper corner 0
                    *free = BOX::new([free_min.0+EPS, new_max.1+EPS], [new_max.0-EPS, free_max.1-EPS]);
                    //right
                    [BOX::new([new_max.0+EPS, free_min.1+EPS], [free_max.0-EPS, free_max.1-EPS]),
                    // right side center 2
                    BOX::new([free_min.0+EPS, free_min.1+EPS], [new_max.0-EPS, new_min.1-EPS])]
                }
                // new overlaps from up
                [_,true,true,_] => {
                    // left
                    *free = BOX::new([free_min.0+EPS, free_min.1+EPS], [new_min.0-EPS, free_max.1-EPS]);
                    // middle

                    [BOX::new([new_min.0+EPS, free_min.1+EPS], [new_max.0-EPS, new_min.1-EPS]),

                    // right
                    BOX::new([new_max.0+EPS, free_min.1+EPS], [free_max.0-EPS, free_max.1-EPS])]

                }

                // new overlaps from the right
                [_,_,true,true] => {
                    // left
                    *free = BOX::new([free_min.0+EPS, free_min.1+EPS], [new_min.0-EPS, free_max.1-EPS]);
                    // right upper corner 1

                    [BOX::new([new_min.0+EPS, new_max.1+EPS], [free_max.0-EPS, free_max.1-EPS]),
                    //right lower corner 2
                        BOX::new([new_min.0+EPS, free_min.1+EPS], [new_max.0-EPS, new_min.1-EPS])]

                }
                // new overlaps from down
                [true,_,_,true] => {
                    // left
                    *free = BOX::new([free_min.0+EPS, free_min.1+EPS], [new_min.0-EPS, free_max.1-EPS]);
                    // middle

                        [BOX::new([new_min.0+EPS, new_max.1+EPS], [new_max.0-EPS, free_max.1-EPS]),
                    // right
                        BOX::new([new_max.0+EPS, free_min.1+EPS], [free_max.0-EPS, free_max.1-EPS])]

                }
                    _ => {unreachable!()}
                }
                // new overlaps from the left
            }



fn identify_intersection_case(
    free: BOX,
    new_min: (f32, f32),
    new_max: (f32, f32),
) -> [bool; 4] {
    let new_verts_in_free = [
        free.contains_in(0, new_min.0) && free.contains_in(1, new_min.1),
        free.contains_in(0, new_min.0) && free.contains_in(1, new_max.1),
        free.contains_in(0, new_max.0) && free.contains_in(1, new_max.1),
        free.contains_in(0, new_max.0) && free.contains_in(1, new_min.1),
    ];
    new_verts_in_free
}

fn cut_space(
    free_space: &mut set::BBoxSet<BOX, usize>,
    intersected: Vec<usize>,
    new:  BOX,
    start_idx: &mut std::ops::RangeFrom<usize>,
) {
    let mut tokill = vec![];

    for i in &intersected {
        dbg!(i);
        let (free, _fsp_index) = free_space.boxes.get_mut(*i).unwrap();

        let free_min = (free.lo(0), free.lo(1));
        let free_max = (free.hi(0), free.hi(1));

        let new_min = (new.lo(0), new.lo(1));
        let new_max = (new.hi(0), new.hi(1));

        let new_verts_in_free= identify_intersection_case(*free, new_min, new_max);
        let free_verts_in_new = identify_intersection_case(new, free_min, free_max);

        let new_in_free_count = new_verts_in_free.iter().map(|&e| e as u8).sum();
        let free_in_new_count = free_verts_in_new.iter().map(|&e| e as u8).sum();

        match (new_in_free_count, free_in_new_count) {
            (0, 4) => {
                // new entirely contains free,  kill free completely
                *free = BOX::new([f32::MAX, f32::MAX], [f32::MAX, f32::MAX]);
                tokill.push(i)
            }
            (4, 0) => {//free entirely contains new, break free into 4 segments
                // left
                 *free = BOX::new([free_min.0+EPS, free_min.1+EPS], [new_min.0-EPS, free_max.1-EPS]);
                // bottom
                 free_space.push(
                            start_idx.next().unwrap(),
                            BOX::new([new_min.0+EPS, new_max.1+EPS], [new_max.0-EPS, free_max.1-EPS]),
                        );
                //top
                 free_space.push(
                    start_idx.next().unwrap(),
                    BOX::new([new_min.0+EPS, free_min.1+EPS], [new_max.0-EPS, new_min.1-EPS]),
                );
                // right
                free_space.push(
                            start_idx.next().unwrap(),
                            BOX::new([new_max.0+EPS, free_min.1+EPS], [free_max.0-EPS, free_max.1-EPS]),
                        );
            }
            (1, 1) => {

                let rotation = new_verts_in_free.iter().position(|&e| e).unwrap();
                let b = one_vertex_intersection( free, free_min, free_max, new_min, new_max, rotation);
                 println!(" BOX Ppushed to free space{:?}", b);
                 free_space.push(start_idx.next().unwrap(),b);
            }

            (2, 0) => {
                let space = two_vertex_intersection_subdivision( free,  new_min, new_max, free_min, free_max, new_verts_in_free);

                for i in space{
                    free_space.push(start_idx.next().unwrap(),i);
                }
                }

            (0, 2) => {
                 let space = two_vertex_intersection_subdivision( free,  free_min, free_max, new_min, new_max, free_verts_in_new);
                    for i in space{
                        free_space.push(start_idx.next().unwrap(),i);
                    }
            }
            (3, _) => {
                unreachable!()
            }
            (_, 3) => {
                unreachable!()
            }

            (_, _) => {
                unreachable!()
            }
        }
    }
    //amount of vert intersected free space
    for i in tokill {
        free_space.boxes.remove(*i);
    }
    // start_idx.next();
    // free_space.boxes.remove(0);
    free_space.sort();
}

fn one_vertex_intersection(free: &mut BOX, free_min: (f32, f32), free_max: (f32, f32),
                           new_min: (f32, f32), new_max: (f32, f32), rotation:usize)->BOX
{
// One vertex intersection
    match rotation {
        0 => {
            println!("CASE 0");
            // Left lower corner intersection
            //left
            *free = BOX::new([free_min.0+EPS, new_max.1+EPS], [new_max.0-EPS, free_max.1-EPS]);

            //right
            BOX::new([new_max.0+EPS, free_min.1+EPS], [free_max.0-EPS, free_max.1-EPS])
            //Checked
        }

        1 => {
             println!("CASE 1");
            // Left upper corner intersection

            // left
            *free = BOX::new([free_min.0+EPS, free_min.1+EPS], [new_max.0-EPS, new_min.1-EPS]);

            // right

                BOX::new([new_max.0+EPS, free_min.1+EPS], [free_max.0-EPS, free_max.1-EPS])

        }
        2 => {
             println!("CASE 2");
            //Right upper corner intersection

            // Left
            *free = BOX::new([free_min.0+EPS, free_min.1+EPS], [new_min.0-EPS, free_max.1-EPS]);

            // Right

                BOX::new([new_min.0+EPS, free_min.1+EPS], [free_max.0-EPS, new_min.1-EPS])

        }
        3 => {
             println!("CASE 3");
            //Right lower corner

            // Left lower corner
            *free = BOX::new([free_min.0+EPS, free_min.1+EPS], [new_min.0-EPS, free_max.1-EPS]);

            // Left upper corner box

                BOX::new([new_min.0+EPS, new_max.1+EPS], [free_max.0-EPS, free_max.1-EPS])
        }
        _ => {unreachable!()}
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use box_intersect_ze::boxes::BBox;


    use plotters::prelude::*;
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
        if num_inters == 0 {
            return;
        }
        //println!("inters array is {:?}", inters);
        //println!("free space before {:?}", free_space.boxes);
        cut_space(&mut free_space, inters, new, &mut index_alloc);
        free_space.sort();
        //println!("free space after {:?}", free_space.boxes);
        {
            let mut res = vec![];
            intersect_scan(&free_space, &free_space, &mut res);
            //assert_eq!(res.len(), 0, "free space should not have self-intersections");
            let inters = intersection_check(&free_space, new);
            dbg!(&inters);
           // assert_eq!(inters.len(), 0, "free space should not intersect new after cut is done");
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
        let  cases = vec![
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
        let  cases = vec![
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
        let  cases = vec![
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
