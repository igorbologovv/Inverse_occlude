use box_intersect_ze::*;

pub type BOX = boxes::Box2Df32;

pub enum OcclusionStatus{
    Occluded,
    PartiallyVisible
}

pub struct OcclusionBuffer{
   pub free_space: set::BBoxSet<BOX, usize>,
   //this is silly but library wants this to store the box for inters check
   new_box: set::BBoxSet<BOX, usize>,
   box_idx_alloc: std::ops::RangeFrom<usize>,
   occlusion_status: Vec<(usize,usize)>,
}

impl OcclusionBuffer{
pub fn new()->Self{
    OcclusionBuffer { free_space: set::BBoxSet::new(), new_box: set::BBoxSet::new(),
    box_idx_alloc: 1..,  occlusion_status:Vec::with_capacity(16)}
}

pub fn check_a_box(&mut self, new:BOX)->OcclusionStatus
{
    self.new_box.clear();
    self.occlusion_status.clear();
    self.new_box.push(usize::MAX - 1, new);
    box_intersect_ze::intersect_scan(self.free_space, &self.new_box, &mut self.occlusion_status);

    if self.occlusion_status.is_empty(){
        OcclusionStatus::Occluded
    }
    else {
        OcclusionStatus::PartiallyVisible
    }
}

fn add_freespace_box(&mut self, b:BOX){

    self.free_space.push(self.box_idx_alloc.next().unwrap(), b)
}

/// Adds box that was last passed into check_a_box
pub fn add_last_box(&mut self){
    //take stuff from self.new_box
}

}
/** Checks if new box intersects free space.
Returns vec of indices of free space regions intersected.
 */
fn occlusion_check(free_space: &set::BBoxSet<BOX, usize>, new: BOX) -> Vec<usize> {
    let mut newset = set::BBoxSet::new();
    newset.push(usize::MAX - 1, new);
    let mut result = Vec::new();

    box_intersect_ze::intersect_scan(free_space, &newset, &mut result);
    result.iter().map(|e| e.0).collect()
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

    for i in intersected{

    }
    // start_idx.next();
    // free_space.boxes.remove(0);
    free_space.sort();
}

fn plot_intersections() {}

#[cfg(test)]
mod tests {
    use crate::*;
    use box_intersect_ze::boxes::BBox;
    use box_intersect_ze::*;
    use plotters::prelude::*;

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
    /* #[test]
    fn bla() -> Result<(), Box<dyn std::error::Error>> {
        let root = BitMapBackend::new("plotters-doc-data.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE)?;
        let mut chart = ChartBuilder::on(&root)
            .caption("y=x^2", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(-1f32..1f32, -0.1f32..1f32)?;

        chart.configure_mesh().draw()?;

        chart
            .draw_series(LineSeries::new(
                (-50..=50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)),
                &RED,
            ))?
            .label("y = x^2")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

        let r = Rectangle::new([(0., 0.),(0.5,0.5) ], &RED);
        let r2 = Rectangle::new([(0., 0.1),(0.5,0.5) ], &RED);
        let t = Text::new("Kill me please", (0., 0.), ("sans-serif", 10).into_font());
        chart.draw_series([r, r2]);
        chart.draw_series([t]);
        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;

        root.present()?;

        Ok(())
    }*/

    fn plotboxes(free_space: &set::BBoxSet<BOX, usize>, new: BOX, name: &str) {
        let mut backend = SVGBackend::new(name, (MAX_PIX as u32, MAX_PIX as u32));
        let style = {
            let F = SVGBackend::new("/dev/null", (150, 150)).into_drawing_area();
            ("sans-serif", 15.0, &BLUE).into_text_style(&F)
        };

        for (b, i) in &free_space.boxes {
            let (lo, hi) = project_coords(*b);
            backend.draw_rect(lo, hi, &BLUE, false).unwrap();
            backend
                .draw_text(&*i.to_string(), &style, lo)
                .expect("TODO: panic message")
        }
        let (lo, hi) = project_coords(new);
        backend.draw_rect(lo, hi, &RED, false).unwrap();
        backend.present().unwrap();
    }

    fn test_inner(free: BOX, new: BOX, name: String, num_inters: usize) {
        let mut index_alloc = 1..;
        let mut free_space = set::BBoxSet::<BOX, usize>::new();
        free_space.push(index_alloc.next().unwrap(), free);
        plotboxes(&free_space, new, &(name.clone() + "__before.svg"));

        let inters = occlusion_check(&free_space, new);
        assert_eq!(inters.len(), num_inters);
        cut_space(&mut free_space, inters, new, &mut index_alloc);
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

    fn better_name(s:&str)->String{
        s.to_string().split("::").last().unwrap().to_string()
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
        test_inner(free, new, format!("hole/{} {}", &better_name(function_name!()), 1), 1);
    }
}
