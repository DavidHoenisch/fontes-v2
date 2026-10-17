//! NativeActivity content edges and surface dimensions are physical pixels.

/// Return left/top/right/bottom margins in egui points. An empty content rectangle
/// is normal during window startup; stale geometry can also occur during rotation.
pub(super) fn content_insets(
    content: [i32; 4],
    surface: [i32; 2],
    pixels_per_point: f32,
) -> Option<[f32; 4]> {
    let [left, top, right, bottom] = content;
    let [width, height] = surface;
    if !pixels_per_point.is_finite()
        || pixels_per_point <= 0.0
        || width <= 0
        || height <= 0
        || left < 0
        || top < 0
        || left >= right
        || top >= bottom
        || right > width
        || bottom > height
    {
        return None;
    }
    Some([
        left as f32 / pixels_per_point,
        top as f32 / pixels_per_point,
        (width - right) as f32 / pixels_per_point,
        (height - bottom) as f32 / pixels_per_point,
    ])
}

#[cfg(test)]
mod tests {
    use super::content_insets;

    #[test]
    fn system_bars_and_landscape_cutouts_convert_at_the_current_scale() {
        assert_eq!(
            content_insets([0, 72, 1080, 2328], [1080, 2400], 3.0),
            Some([0.0, 24.0, 0.0, 24.0])
        );
        assert_eq!(
            content_insets([96, 0, 2328, 1080], [2400, 1080], 3.0),
            Some([32.0, 0.0, 24.0, 0.0])
        );
        // User zoom is included in pixels_per_point, not applied a second time.
        assert_eq!(
            content_insets([0, 72, 1080, 2328], [1080, 2400], 4.0),
            Some([0.0, 18.0, 0.0, 18.0])
        );
    }

    #[test]
    fn already_resized_windows_do_not_receive_duplicate_padding() {
        assert_eq!(
            content_insets([0, 0, 1080, 2200], [1080, 2200], 3.0),
            Some([0.0; 4])
        );
        assert_eq!(
            content_insets([0, 72, 1080, 1500], [1080, 2400], 3.0),
            Some([0.0, 24.0, 0.0, 300.0])
        );
    }

    #[test]
    fn startup_and_stale_geometry_are_not_treated_as_screen_covering_insets() {
        for content in [
            [0; 4],
            [0, 72, 2400, 1080],
            [-1, 0, 1080, 2400],
            [0, 2500, 1080, 2400],
        ] {
            assert!(content_insets(content, [1080, 2400], 3.0).is_none());
        }
        for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(content_insets([0, 72, 1080, 2328], [1080, 2400], scale).is_none());
        }
    }
}
