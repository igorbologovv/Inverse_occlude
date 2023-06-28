use box_intersect_ze::boxes::{BBox, Box2Df32};
use box_intersect_ze::set::BBoxSet;
use box_intersect_ze::*;
use pyo3::*;
const EPS: f32 = 0.0;//00001;
const NOWHERE: f32 = f32::MAX;

pub type BOX = boxes::Box2Df32;

#[pyclass]
#[derive(Clone)]
pub struct PyOcclusionBuffer {
    occl_buf: OcclusionBuffer,
}

#[pymethods]
impl PyOcclusionBuffer {
    #[new]
    pub fn new(bot: [f32; 2], top: [f32; 2]) -> Self {
        Self {
            occl_buf: OcclusionBuffer::new(BOX::new(bot, top)),
        }
    }

    pub fn copy(&self) -> PyOcclusionBuffer {
        self.clone()
    }

    /// check if a new box intersects free space
    pub fn check_a_box(&mut self, new: ([f32; 2], [f32; 2])) -> bool {
        match self.occl_buf.check_a_box(BOX::new(new.0, new.1)) {
            OcclusionStatus::Occluded => false,
            OcclusionStatus::PartiallyVisible => true,
        }
    }

    /// Adds box that was last passed into check_a_box
    pub fn add_last_box(&mut self) {
        self.occl_buf.add_last_box();
    }

    /// add multiple boxes into zbuffer while cutting space for each one
    pub fn add_box_set(&mut self, boxes: Vec<([f32; 2], [f32; 2])>) {

        let mut box_vec = vec![];
        for b in &boxes {
            box_vec.push(BOX::new(b.0, b.1));
        }
        self.occl_buf.add_box_set(box_vec);
    }
}
use pyo3::types::PyModule;
#[pymodule]
fn aabb_occlusion_culling(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyOcclusionBuffer>()?;
    Ok(())
}

pub enum OcclusionStatus {
    Occluded,
    PartiallyVisible,
}

#[derive(Clone)]
pub struct OcclusionBuffer {
    pub free_space: BBoxSet<BOX, usize>,
    overall_bound_box: BOX,
    //this is silly but library wants this to store the box for inters check
    new_box: BBoxSet<BOX, usize>,
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
                let mut s = BBoxSet::with_capacity(256);
                s.push(0, freespace);
                s
            },
            overall_bound_box: freespace,
            new_box: BBoxSet::new(),
            box_idx_alloc: 1..,
            occlusion_status: Vec::with_capacity(128),
            dead_boxes: vec![],
        }
    }

    /// check if a new box intersects free space
    pub fn check_a_box(&mut self, new: BOX) -> OcclusionStatus {
        assert!(self.overall_bound_box.contains(new), "New box should not be out of bounds of free space");
        self.new_box.clear();
        self.occlusion_status.clear();
        self.new_box.push(usize::MAX - 1, new);
        intersect_scan_idx(
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
        debug_assert!(!self.new_box.empty());

        let newbox = self.new_box.boxes[0].0;
        println!("Created newbox: self.new_box.boxes[0].0 {:?}", newbox);
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

/** Given vector of free space boxes and vec of indices of free space regions intersected by box new,
breaks up boxes in free space until everything is correct again.
Returns number of new boxes added to free space set
 */

fn two_vertex_intersection_subdivision(  
    new_min: (f32, f32),
    new_max: (f32, f32),
    free_min: (f32, f32),
    free_max: (f32, f32),
    new_verts_in_free: [bool; 4],
    reverse: bool,
) -> (BOX, Option<[BOX; 2]>) {
    // Two vertex intersection
    //     free.contains_in(0, new_min.0) && free.contains_in(1, new_min.1), left lower corner
    //     free.contains_in(0, new_min.0) && free.contains_in(1, new_max.1), left upper corner
    //     free.contains_in(0, new_max.0) && free.contains_in(1, new_max.1), right upper corner
    //     free.contains_in(0, new_max.0) && free.contains_in(1, new_min.1), right lower corner

    match new_verts_in_free {
        [true, true, _, _] => {
            if reverse {
                return (BOX::new(
                    [free_max.0 + EPS, new_min.1 + EPS],
                    [new_max.0 - EPS, new_max.1 - EPS],
                ), None);                
            } 
                (BOX::new(
                    [free_min.0 + EPS, free_min.1 + EPS],
                    [new_min.0 - EPS, free_max.1 - EPS],
                ),                
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
                ]))
            
        }

        [_, true, true, _] => {
            if reverse {
                return (BOX::new(
                    [new_min.0 + EPS, new_min.1 + EPS],
                    [new_max.0 - EPS, free_min.1 - EPS],
                ), None);
                               
            }
            // left
            (BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_min.0 - EPS, free_max.1 - EPS],
            ),
           
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
            ]))
        }

        // new overlaps from the right
        [_, _, true, true] => {
            if reverse {
                return ( BOX::new(
                    [new_min.0 + EPS, new_min.1 + EPS],
                    [free_min.0 - EPS, new_max.1 - EPS],
                ),           
                None);
            }

            (BOX::new(
                [free_min.0 + EPS, new_max.1 + EPS],
                [new_max.0 - EPS, free_max.1 - EPS],
            ),                        
            Some([
                BOX::new(
                    [new_max.0 + EPS, free_min.1 + EPS],
                    [free_max.0 - EPS, free_max.1 - EPS],
                ),
                BOX::new(
                    [free_min.0 + EPS, free_min.1 + EPS],
                    [new_max.0 - EPS, new_min.1 - EPS],
                ),
            ]))
            ////
        }
        // new overlaps from down
        [true, _, _, true] => {
            if reverse {
                return (BOX::new(
                    [new_min.0 + EPS, free_max.1 + EPS],
                    [new_max.0 - EPS, new_max.1 - EPS],
                ),                         
                 None);
            }

            // left
            ( BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_min.0 - EPS, free_max.1 - EPS],
            ),
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
            ]))
            ////
        }
        _ => {
            println!("Something unpredictable happened");
            unreachable!()
        }
    }
}



trait BoxExtensions{
    /// Checks which vertices are contained in self.
    /// If none are contained, array of false is returned.
    /// ordering of returned values is [left lower ,left upper, right upper ,right lower]
    fn identify_intersection_case(
        &self,
        min_vertex: (f32, f32),
        max_vertex: (f32, f32),
    ) -> [bool; 4] ;
    /// checks if b is fully contained in self
    fn contains(&self, b:Self)->bool;
    
    /// Checks if self has zero area
    fn is_empty(&self)->bool;
}

impl BoxExtensions for BOX{
    

fn identify_intersection_case(
   &self,
    min_vertex: (f32, f32),
    max_vertex: (f32, f32),
) -> [bool; 4] {
    [
        self.contains_in(0, min_vertex.0) && self.contains_in(1, min_vertex.1),
        self.contains_in(0, min_vertex.0) && self.contains_in(1, max_vertex.1),
        self.contains_in(0, max_vertex.0) && self.contains_in(1, max_vertex.1),
        self.contains_in(0, max_vertex.0) && self.contains_in(1, min_vertex.1),
    ]
}
    fn contains(&self, b:Self)->bool{
        self.identify_intersection_case(
            (b.lo(0), b.lo(1)),
            (b.hi(0), b.hi(1))).iter().all(|&e|e)
    }
    
    fn is_empty(&self)->bool {
         (self.lo(0) == self.hi(0)) || (self.lo(1) == self.hi(1))            
    }
}

///
fn cut_space(
    mut free_space: &mut BBoxSet<BOX, usize>,
    mut tokill: &mut Vec<usize>,
    intersected: &[(usize, usize)],
    new: BOX,
    start_idx: &mut std::ops::RangeFrom<usize>,
) {
    fn maybe_push(tokill: &mut Vec<usize>, freesp: &mut BBoxSet<BOX, usize>, b: BOX, i: usize) {
        // Do not insert empty boxes
        dbg!(i,b);
        if b.is_empty(){
            println!("Empty BOX too bad {:?}", b);
            return;
        }
        match tokill.pop() {
            Some(v) => freesp.boxes[v] = (b, i), 
            None => {
                freesp.push(i, b);
            }
        }
    }
    
    
    for &(i, _) in intersected.iter() {
        let (free, _fsp_index) = free_space 
            .boxes
            .get_mut(i)
            .expect("Intersected index incorrect!");

        let free_min = (free.lo(0), free.lo(1));
        let free_max = (free.hi(0), free.hi(1));

        let new_min = (new.lo(0), new.lo(1));
        let new_max = (new.hi(0), new.hi(1));

        let new_verts_in_free = free.identify_intersection_case(new_min, new_max);
        let free_verts_in_new = new.identify_intersection_case(free_min, free_max);

        let new_in_free_count = new_verts_in_free.iter().map(|&e| e as u8).sum();
        let free_in_new_count = free_verts_in_new.iter().map(|&e| e as u8).sum();

        // Delete the now invalid free box by moving it into "Nowhere"
        *free = BOX::new([NOWHERE, NOWHERE], [NOWHERE, NOWHERE]);
        // Add it to freelist for memory reuse
        tokill.push(i);  
        
        match (new_in_free_count, free_in_new_count) {
            (0, 4) => {
                       
            }
            (4, 0) => {
                //free entirely contains new, break free into 4 segments
                // left
                maybe_push(
                    &mut tokill,
                    &mut free_space,
                    BOX::new(
                    [free_min.0 + EPS, free_min.1 + EPS],
                    [new_min.0 - EPS, free_max.1 - EPS],
                    ),
                    start_idx.next().unwrap(),
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
                maybe_push(
                    &mut tokill,
                    &mut free_space,                    
                    BOX::new(
                        [new_min.0 + EPS, free_min.1 + EPS],
                        [new_max.0 - EPS, new_min.1 - EPS],
                    ),
                    start_idx.next().unwrap()
                );
                // right
                 maybe_push(
                    &mut tokill,
                    &mut free_space,    
                    BOX::new(
                        [new_max.0 + EPS, free_min.1 + EPS],
                        [free_max.0 - EPS, free_max.1 - EPS],
                    ),
                    start_idx.next().unwrap()
                );
            }
            (1, 1) => {
                let rotation = new_verts_in_free.iter().position(|&e| e).unwrap();
                let (b1, b2) =
                    one_vertex_intersection(free_min, free_max, new_min, new_max, rotation);
                 maybe_push(
                    &mut tokill,
                    &mut free_space, 
                    b1,
                    start_idx.next().unwrap()
                );  
                 maybe_push(
                    &mut tokill,
                    &mut free_space, 
                    b2,
                    start_idx.next().unwrap()
                );  
            }

            (2, 0) => {
                let (b, space) = two_vertex_intersection_subdivision(                    
                    new_min,
                    new_max,
                    free_min,
                    free_max,
                    new_verts_in_free,
                    false,
                );
                maybe_push(
                            &mut tokill,
                            &mut free_space, b, start_idx.next().unwrap());
                           
                match space {
                    None => {}
                    Some(boxes) => {
                        for b in boxes {
                             maybe_push(
                            &mut tokill,
                            &mut free_space, b, start_idx.next().unwrap());
                           
                        }
                    }
                }
            }

            (0, 2) => {
                let (b, space) = two_vertex_intersection_subdivision(                 
                    free_min,
                    free_max,
                    new_min,
                    new_max,
                    free_verts_in_new,
                    true,
                );
                        maybe_push(
                        &mut tokill,
                        &mut free_space, b, start_idx.next().unwrap());
                match space {
                    None => {}
                    Some(boxes)=> {
                        for b in boxes {
                              maybe_push(
                            &mut tokill,
                            &mut free_space, b, start_idx.next().unwrap());
                        }
                    }
                }
            }
            (3, num_vert) => {
                println!("3 {num_vert} HAPPENED ");
                let mut new_space = set::BBoxSet::<BOX, usize>::new(); 
                        new_space.push(i, new);
                        plotboxes(&free_space, &new_space, "no_test_draw" );
                unreachable!()
            }
            (num_vert, 3) => {
                
                let mut new_space = set::BBoxSet::<BOX, usize>::new(); 
                        new_space.push(i, new);
                        plotboxes(&free_space, &new_space, "no_test_draw" );
                println!("{num_vert} 3 HAPPENED");
                unreachable!()
            }
            // intersection occurs, but no vertices lie inside other box
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
                        maybe_push(
                            &mut tokill,
                            &mut free_space, 
                            BOX::new(
                                        [new_max.0 + EPS, free_min.1 + EPS],
                                        [free_max.0 - EPS, free_max.1 - EPS],
                                    ),
                            start_idx.next().unwrap()
                        );                          
                    }
                    (false, true) => {
                        *free = BOX::new(
                            [free_min.0 + EPS, new_max.1 + EPS],
                            [free_max.0 - EPS, free_max.1 - EPS],
                        );
                        if free.is_empty(){
                            
                        }
                        // right
                        maybe_push(
                            &mut tokill,
                            &mut free_space, 
                            BOX::new(
                                [free_min.0 + EPS, free_min.1 + EPS],
                                [free_max.0 - EPS, new_min.1 - EPS],
                            ),
                            start_idx.next().unwrap()
                        );                            
                    }
                    (false, false) => {
                         println!("You don't have to to visit a doctor");
                        unreachable!()
                    }
                    (true, true ) => {
                        println!("You might want to visit a doctor");
                        unreachable!()
                    }
                
                }
            }

            (a, b) => {

                println!("{a} {b} HAPPENED ");
                let mut new_space = set::BBoxSet::<BOX, usize>::new(); 
                        new_space.push(i, new);
                        plotboxes(&free_space, &new_space, "no_test_draw" );
                unreachable!()
            }
        }
    }

    // start_idx.next();
    // free_space.boxes.remove(0);
    free_space.sort();
}

fn one_vertex_intersection(    
    free_min: (f32, f32),
    free_max: (f32, f32),
    new_min: (f32, f32),
    new_max: (f32, f32),
    rotation: usize,
) -> (BOX, BOX) {
    // One vertex intersection

    match rotation {
        //Right upper corner intersection
        0 => {            
            // Left
            (
                BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_min.0 - EPS, free_max.1 - EPS],
                )  ,         
                // Right
                BOX::new(
                    [new_min.0 + EPS, free_min.1 + EPS],
                    [free_max.0 - EPS, new_min.1 - EPS],
                )
            )
        }
        //Right lower corner
        1 => {
            

            // Left lower corner
            (BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_min.0 - EPS, free_max.1 - EPS],
            ),
           
            // Left upper corner box

            BOX::new(
                [new_min.0 + EPS, new_max.1 + EPS],
                [free_max.0 - EPS, free_max.1 - EPS],
            )
            )
        }
        // Left lower corner intersection
        2 => {
            //////////

            
            //left

           (BOX::new(
                [free_min.0 + EPS, new_max.1 + EPS],
                [new_max.0 - EPS, free_max.1 - EPS],
            ),

            //right
            BOX::new(
                [new_max.0 + EPS, free_min.1 + EPS],
                [free_max.0 - EPS, free_max.1 - EPS],
            ))
            //Checked
            ////////////
        }
        3 => {
            ////

            // Left upper corner intersection

            // left
           (BOX::new(
                [free_min.0 + EPS, free_min.1 + EPS],
                [new_max.0 - EPS, new_min.1 - EPS],
            ),

            // right

            BOX::new(
                [new_max.0 + EPS, free_min.1 + EPS],
                [free_max.0 - EPS, free_max.1 - EPS],
            ))
            /////
        }
        i => {
            dbg!("How the hell", i);
            unreachable!()
        }
    }
}

 


    use plotters::prelude::*;
    use plotters::style::full_palette::BLUE_50;
    
    const MAX_PIX: i32 = 256;
fn plotboxes(free_space: &set::BBoxSet<BOX, usize>, new: &set::BBoxSet<BOX, usize>, name: &str) {
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
        let style = {
            let f = SVGBackend::new("/dev/null", (150, 150)).into_drawing_area();
            ("sans-serif", 15.0, &RED).into_text_style(&f)
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

        let res = (
            ((b.lo(0) * 100.) as i32 + 100, (b.lo(1) * 100.) as i32 + 100),
            ((b.hi(0) * 100.) as i32 + 100, (b.hi(1) * 100.) as i32 + 100),
        );

        Some(res)
    }




#[cfg(test)]
mod tests {
    use crate::*;
    use box_intersect_ze::boxes::BBox;

    //use plotters::prelude::*;
    use plotters::style::full_palette::BLUE_50;
    use stdext::function_name;
   
    
    fn test_inner_multiple(free: &Vec<BOX>, new: &Vec<BOX>, directory: &str, name: &str, expect_num_inters: usize) { 
        assert_ne!(directory.len(), 0,"Directory should not be empty!");
        let name = directory.to_owned() + "/" + &better_name(&name);        
        
        let mut index_alloc_new = 1..; 
        let mut new_space = set::BBoxSet::<BOX, usize>::new();        
        for v in new {
            new_space.push(index_alloc_new.next().unwrap(), *v);
        }
        new_space.sort();
        
        let mut index_alloc = 1..;        
        let mut free_space = set::BBoxSet::<BOX, usize>::new();        
        for v in free {
            free_space.push(index_alloc.next().unwrap(), *v);
        }
        free_space.sort();

        
        plotboxes(&free_space, &new_space, &(name.clone() + "__before.svg"));
        let mut num_inters = 0;
        for (i, &newbox) in new.iter().enumerate(){            
            let inters = intersection_check(&free_space, newbox);
            
            println!("inters array is {:?}", inters);
            println!("free space before {:?}", free_space.boxes);
            let mut tokill = vec![];
            cut_space(&mut free_space, &mut tokill, &inters, newbox, &mut index_alloc);
            num_inters += inters.len();
            free_space.sort();
            println!("free space after {:?}", free_space.boxes);
            plotboxes(&free_space, &new_space, &format!("{name}_after{i}.svg"));
            {
                 let mut res = vec![];
                 intersect_scan(&free_space, &free_space, &mut res);
                 if res.len() > 0{
                     dbg!(res);
                     panic!("free space should not have self-intersections");
                 }                
                
                let inters = intersection_check(&free_space, newbox);
                if inters.len() >0{
                    dbg!(&free_space.boxes);
                    dbg!(newbox);
                    panic!("free space should not intersect new after cut is done");
                }
            }
            println!("All checks OK Free space{:?} NEW: {:?}", free_space.boxes, new_space.boxes);                        
        }       
        assert_eq!(expect_num_inters, num_inters);
    }
    
    fn test_inner(free: BOX, new: BOX, directory: &str, name: &str, num_inters: usize) {
        test_inner_multiple(&vec![free], &vec![new], directory, name, num_inters);
    }

    #[test]
    pub fn test_multiple_free_space() {
        let free = vec![BOX::new([0., 0.], [0.5, 1.]), BOX::new([0.5, 0.], [1., 1.])];

        let new = vec![BOX::new([0.2, 0.2], [0.7, 0.7])]; //inside the empty
        test_inner_multiple(&free, &new, ".", function_name!(), 2);
    }

    #[test]
    pub fn test_ustroi_destroy() {
        let   free = vec![BOX::new([0., 0.], [1., 1.])];

        let new = vec![BOX::new([0.3, 0.3], [0.6, 0.6]),BOX::new([0.6, 0.3], [0.9, 0.9]) ]; //inside the empty
        
        
        test_inner_multiple(&free, &new, ".", &function_name!(), 2);   

    }
    
    ///Two boxes that touch in their very corner
    #[test]
    pub fn test_destroy_corner_touch() {
        
        let free = vec![BOX::new([0., 0.], [1., 1.])];

        let new = vec![BOX::new([0.3, 0.3], [0.6, 0.6]), BOX::new([0.6, 0.6], [0.9, 0.9]) ]; 
        
        
        test_inner_multiple(&free, &new, ".", function_name!(), 2);   

    }

    
    
    #[test]
    pub fn test_full_overlap() {
        let free = BOX::new([0., 0.], [1., 1.]); // base
        let new = BOX::new([0.2, 0.2], [0.7, 0.7]); //inside the empty
        test_inner(
            free,
            new,
            "test_plots", function_name!(),
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
                "test_corner_overlap", &format!("{} {}",function_name!(), i),
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
                "two_vertex_overlap", &format!("{} {}",function_name!(), i),
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
                "vert_horizont_div",  &format!("{} {}", function_name!(), i),
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
                "height_width",  &format!("{} {}",function_name!(), i),
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
            "free_in_new", function_name!(), 
            1,
        );
    }

      
    
    
    fn plotboxes(free_space: &set::BBoxSet<BOX, usize>, new: &set::BBoxSet<BOX, usize>, name: &str) {
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
        let style = {
            let f = SVGBackend::new("/dev/null", (150, 150)).into_drawing_area();
            ("sans-serif", 15.0, &RED).into_text_style(&f)
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

    /** Checks if new box intersects free space.
    Returns vec of indices of free space regions intersected.
     */
    fn intersection_check(free_space: &set::BBoxSet<BOX, usize>, new: BOX) -> Vec<(usize, usize)> {
        // create set for comparing two sets intersection
        let mut new_set = set::BBoxSet::new();
        new_set.push(usize::MAX - 1, new);       
        let mut result = Vec::new();                
        intersect_scan_idx(free_space, &new_set, &mut result);           
        result
    }
}
