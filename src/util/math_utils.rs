use glam::Vec3;

pub fn normalize_to_white(v: Vec3) -> Vec3 {
    // This sets the max val to 1.0 and scales the rest of the values along with it.
    // This makes the overall color as bright as possible while maintaining the
    // overall hue ratio
    let max_val = v.max_element();
    if max_val > 0.0 {
        v / max_val
    } else {
        v
    }
}
