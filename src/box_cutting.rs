use box_intersect_ze::boxes::BBox;
pub const NOWHERE: f32 = f32::MAX;
use box_intersect_ze::*;
pub type BOX = boxes::Box2Df32;
pub const EPS: f32 = 0.0; //00001;

pub trait BoxExtensions {
    /// Checks which vertices are contained in self.
    /// If none are contained, array of false is returned.
    /// ordering of returned values is [left lower ,left upper, right upper ,right lower]
    fn identify_intersection_case(
        &self,
        min_vertex: (f32, f32),
        max_vertex: (f32, f32),
    ) -> [bool; 4];
    /// checks if b is fully contained in self
    fn contains(&self, b: Self) -> bool;

    /// Checks if self has zero area
    fn is_empty(&self) -> bool;
    fn safe_new(a: [f32; 2], b: [f32; 2]) -> Self;
    /// Checks if a given point is strictly inside a given box
    fn contains_point(&self, v: [f32; 2]) -> bool;
}

impl BoxExtensions for BOX {
    fn safe_new(min: [f32; 2], max: [f32; 2]) -> Self {
        assert!(min[0].abs() < 100.0);
        assert!(min[1].abs() < 100.0);
        assert!(max[0].abs() < 100.0);
        assert!(max[1].abs() < 100.0);
        BOX::new(min, max)
    }

    fn contains_point(&self, v: [f32; 2]) -> bool {
        self.lo(0) < v[0] && v[0] < self.hi(0) && self.lo(1) < v[1] && v[1] < self.hi(1)
    }

    fn identify_intersection_case(
        &self,
        min_vertex: (f32, f32),
        max_vertex: (f32, f32),
    ) -> [bool; 4] {
        [
            self.contains_point([min_vertex.0, min_vertex.1]), // left low
            self.contains_point([min_vertex.0, max_vertex.1]), //left up
            self.contains_point([max_vertex.0, max_vertex.1]), // right up
            self.contains_point([max_vertex.0, min_vertex.1]), // right low
        ]
    }
    fn contains(&self, b: Self) -> bool {
        self.identify_intersection_case((b.lo(0), b.lo(1)), (b.hi(0), b.hi(1)))
            .iter()
            .all(|&e| e)
    }

    fn is_empty(&self) -> bool {
        (self.lo(0) == self.hi(0)) || (self.lo(1) == self.hi(1))
    }
}

pub fn one_vertex_intersection(
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
                ),
                // Right
                BOX::new(
                    [new_min.0 + EPS, free_min.1 + EPS],
                    [free_max.0 - EPS, new_min.1 - EPS],
                ),
            )
        }
        //Right lower corner
        1 => {
            // Left lower corner
            (
                BOX::new(
                    [free_min.0 + EPS, free_min.1 + EPS],
                    [new_min.0 - EPS, free_max.1 - EPS],
                ),
                // Left upper corner box
                BOX::new(
                    [new_min.0 + EPS, new_max.1 + EPS],
                    [free_max.0 - EPS, free_max.1 - EPS],
                ),
            )
        }
        // Left lower corner intersection
        2 => {
            //////////

            //left

            (
                BOX::new(
                    [free_min.0 + EPS, new_max.1 + EPS],
                    [new_max.0 - EPS, free_max.1 - EPS],
                ),
                //right
                BOX::new(
                    [new_max.0 + EPS, free_min.1 + EPS],
                    [free_max.0 - EPS, free_max.1 - EPS],
                ),
            )
            //Checked
            ////////////
        }
        3 => {
            ////

            // Left upper corner intersection

            // left
            (
                BOX::new(
                    [free_min.0 + EPS, free_min.1 + EPS],
                    [new_max.0 - EPS, new_min.1 - EPS],
                ),
                // right
                BOX::new(
                    [new_max.0 + EPS, free_min.1 + EPS],
                    [free_max.0 - EPS, free_max.1 - EPS],
                ),
            )
            /////
        }
        i => {
            dbg!("How the hell", i);
            unreachable!()
        }
    }
}

/// Produces subdivided boxes for cases where two vertices of "new" box overlap with "free" box
pub fn two_vertex_intersection_subdivision(
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
                return (
                    BOX::new(
                        [free_max.0 + EPS, new_min.1 + EPS],
                        [new_max.0 - EPS, new_max.1 - EPS],
                    ),
                    None,
                );
            }
            (
                BOX::new(
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
                ]),
            )
        }

        [_, true, true, _] => {
            if reverse {
                return (
                    BOX::new(
                        [new_min.0 + EPS, new_min.1 + EPS],
                        [new_max.0 - EPS, free_min.1 - EPS],
                    ),
                    None,
                );
            }
            // left
            (
                BOX::new(
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
                ]),
            )
        }

        // new overlaps from the right
        [_, _, true, true] => {
            if reverse {
                return (
                    BOX::new(
                        [new_min.0 + EPS, new_min.1 + EPS],
                        [free_min.0 - EPS, new_max.1 - EPS],
                    ),
                    None,
                );
            }

            (
                BOX::new(
                    [free_min.0 + EPS, new_max.1 + EPS],
                    [new_max.0 - EPS, free_max.1 - EPS],
                ),
                Some([
                    BOX::new(
                        [new_max.0 + EPS, free_min.1 + EPS],
                        [free_max.0 - EPS, free_max.1 - EPS],
                    ),
                    BOX::safe_new(
                        [free_min.0 + EPS, free_min.1 + EPS],
                        [new_max.0 - EPS, new_min.1 - EPS],
                    ),
                ]),
            )
            ////
        }
        // new overlaps from down
        [true, _, _, true] => {
            if reverse {
                return (
                    BOX::safe_new(
                        [new_min.0 + EPS, free_max.1 + EPS],
                        [new_max.0 - EPS, new_max.1 - EPS],
                    ),
                    None,
                );
            }

            // left
            (
                BOX::new(
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
                ]),
            )
            ////
        }
        _ => {
            println!("Something unpredictable happened");
            unreachable!()
        }
    }
}
