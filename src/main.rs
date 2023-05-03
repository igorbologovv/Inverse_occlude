use box_intersect_ze::*;
use plotpy::{Curve, Plot};

fn main() {

    //TODO!
    // let result = cut_space();
    // println!("{:?}", result);
    plot_intersections()
}

type BOX = boxes::Box2Df32;
/** Checks if new box intersects free space.
Returns vec of indices of free space regions intersected.
 */
fn occlusion_check(free_space: &set::BBoxSet<BOX, usize>,
                   new:BOX ) -> Vec<usize>
{
    todo!()
}


/** Given vector of free space boxes and vec of indices of free space regions intersected by box new,
breaks up boxes in free space until everything is correct again.
Returns number of new boxes added to free space set
 */
fn cut_space(  free_space: &mut set::BBoxSet<BOX, usize>,
               intersected: Vec<usize> , new: BOX, start_idx:usize )->usize{



    free_space.boxes.remove(42);
    free_space.sort();
    42
}

fn plot_intersections(){
    let quad = BOX::new([0., 0.], [1., 1]); // base

    let case_1 = BOX::new([0.7, -0.2], [1.5, 1.5]); // more than width
    let case_2 = BOX::new([-0.2, 0.2], [0.7, 0.2]);// overlap less than height less than width
    let case_3 = BOX::new([0.2, 0.2], [0.7, 0.7]);//inside the empty
    let case_4 = BOX::new([0.7, -0.3], [1.3, 0.3]);//edge
    let special_case = BOX::new([-0.3, 0.7], [1.3, 0.7]);//overlap less than height more than width


    let mut curve = Curve::new();
    println!("{:?}", case_1 );
    //curve.draw(x, y);
    let mut plot = Plot::new();

}

cfg test
{
fn plotboxes(free_space: &set::BBoxSet<BOX, usize>,new:BOX){

}
fn test_inner(free:BOX, new:BOX){
    plotboxes(...);
    cut_space();
    plotboxes(...);
}
fn test_full_overlap(){
    let case_1 = BOX::new([0.7, -0.2], [1.5, 1.5]);
    test_inner(...);
}
fn test_1vedrtex_overlap_overlap(){
    let case_1 = BOX::new([0.7, -0.2], [1.5, 1.5]);
    for i in 1..4 {
        test_inner(...);
    }
}

}
