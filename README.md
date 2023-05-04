A simple library that does hierarchical occlusion check with AABBs.
One could think of this as a z-buffer but with float boundaries between pixels.

# How this works

This works by encoding "free space" as a set of non-overlapping AABBs, after which the following logic applies:
 - If the new box does not intersect any "free space" box, it is fully occluded
 - If the new box intersects a "free space" box, it is partially visible, and more detailed checks can be done on it 
 - If a new box is added into the set, appropriate free space boxes are broken up and/or removed to match
