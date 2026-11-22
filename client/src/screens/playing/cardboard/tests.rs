use super::cam_dist_in_height;

#[test]
fn camera_stays_between_feet_and_wall_tops() {
    let (pivot, low, high) = (1.1, 0.3, 2.25);
    // level, full distance
    assert_eq!(cam_dist_in_height(pivot, 0.0, low, high, 2.6), 2.6);
    // looking up 30 deg, the eye goes down and stops 0.3 over the feet
    let d = cam_dist_in_height(pivot, -0.5, low, high, 2.6);
    assert!((pivot - 0.5 * d - low).abs() < 1e-5);
    // looking down 45 deg, the eye goes up and stops under the wall tops
    let up = std::f32::consts::FRAC_1_SQRT_2;
    let d = cam_dist_in_height(pivot, up, low, high, 2.6);
    assert!((pivot + up * d - high).abs() < 1e-5);
    // straight down, never past the cap, and no ceiling means the full distance
    assert!((cam_dist_in_height(pivot, 1.0, low, high, 2.6) - 1.15).abs() < 1e-5);
    assert_eq!(cam_dist_in_height(pivot, 1.0, low, f32::INFINITY, 2.6), 2.6);
    // a pivot already out of range gives no room instead of a negative distance
    assert_eq!(cam_dist_in_height(3.0, 0.5, low, high, 2.6), 0.0);
}
