use std::collections::BTreeMap;

/// Active pointer position in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchPoint {
    pub x: f64,
    pub y: f64,
}

/// Output delta produced by [`PinchPanTracker`] on pointer movement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureDelta {
    /// Single-finger drag delta in CSS pixels.
    Pan { dx: f64, dy: f64 },
    /// Two-finger pinch scale ratio (`> 1.0` zoom in, `< 1.0` zoom out),
    /// midpoint anchor in CSS pixels, and midpoint pan delta.
    Pinch {
        scale_ratio: f64,
        center_x: f64,
        center_y: f64,
        pan_dx: f64,
        pan_dy: f64,
    },
}

/// Stateful pointer tracker for smooth pinch-to-zoom and single/dual-finger panning
/// across HTML5 canvases (Logic Analyzer, Oscilloscope, SDR Waterfall).
#[derive(Debug, Clone, Default)]
pub struct PinchPanTracker {
    pointers: BTreeMap<i32, TouchPoint>,
    last_pinch_dist: Option<f64>,
    last_pinch_center: Option<TouchPoint>,
}

impl PinchPanTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_count(&self) -> usize {
        self.pointers.len()
    }

    pub fn clear(&mut self) {
        self.pointers.clear();
        self.last_pinch_dist = None;
        self.last_pinch_center = None;
    }

    pub fn pointer_down(&mut self, pointer_id: i32, x: f64, y: f64) {
        self.pointers.insert(pointer_id, TouchPoint { x, y });
        self.sync_pinch_anchor();
    }

    pub fn pointer_up(&mut self, pointer_id: i32) {
        self.pointers.remove(&pointer_id);
        self.sync_pinch_anchor();
    }

    pub fn pointer_move(&mut self, pointer_id: i32, x: f64, y: f64) -> Option<GestureDelta> {
        let prev = self.pointers.insert(pointer_id, TouchPoint { x, y })?;

        if self.pointers.len() == 1 {
            self.last_pinch_dist = None;
            self.last_pinch_center = None;
            return Some(GestureDelta::Pan {
                dx: x - prev.x,
                dy: y - prev.y,
            });
        }

        if self.pointers.len() >= 2 {
            let (dist, center) = self.current_two_finger_metrics()?;
            let result = match (self.last_pinch_dist, self.last_pinch_center) {
                (Some(prev_dist), Some(prev_center)) if prev_dist > 1.0 && dist > 1.0 => {
                    Some(GestureDelta::Pinch {
                        scale_ratio: dist / prev_dist,
                        center_x: center.x,
                        center_y: center.y,
                        pan_dx: center.x - prev_center.x,
                        pan_dy: center.y - prev_center.y,
                    })
                }
                _ => None,
            };
            self.last_pinch_dist = Some(dist);
            self.last_pinch_center = Some(center);
            return result;
        }

        None
    }

    fn sync_pinch_anchor(&mut self) {
        if self.pointers.len() >= 2 {
            if let Some((dist, center)) = self.current_two_finger_metrics() {
                self.last_pinch_dist = Some(dist);
                self.last_pinch_center = Some(center);
                return;
            }
        }
        self.last_pinch_dist = None;
        self.last_pinch_center = None;
    }

    fn current_two_finger_metrics(&self) -> Option<(f64, TouchPoint)> {
        let mut iter = self.pointers.values();
        let a = *iter.next()?;
        let b = *iter.next()?;
        let dist = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        let center = TouchPoint {
            x: (a.x + b.x) * 0.5,
            y: (a.y + b.y) * 0.5,
        };
        Some((dist, center))
    }
}

/// Applies a zoom factor around a normalized anchor (`0.0..=1.0` within the viewport)
/// while clamping `zoom` to `[min_zoom, max_zoom]` and keeping `pan_norm` in valid bounds.
///
/// Returns `(next_zoom, next_pan_norm)`.
pub fn apply_anchored_zoom(
    current_zoom: f64,
    current_pan_norm: f64,
    scale_factor: f64,
    anchor_norm: f64,
    min_zoom: f64,
    max_zoom: f64,
) -> (f64, f64) {
    let old_zoom = current_zoom.clamp(min_zoom, max_zoom);
    let new_zoom = (old_zoom * scale_factor).clamp(min_zoom, max_zoom);
    let anchor = anchor_norm.clamp(0.0, 1.0);

    let old_visible = 1.0 / old_zoom;
    let new_visible = 1.0 / new_zoom;
    let world_at_anchor = current_pan_norm + anchor * old_visible;
    let max_pan = (1.0 - new_visible).max(0.0);
    let new_pan = (world_at_anchor - anchor * new_visible).clamp(0.0, max_pan);

    (new_zoom, new_pan)
}

/// Applies a horizontal pan delta (in CSS pixels) over a viewport width,
/// clamping `pan_norm` so the window stays within `[0.0, 1.0]`.
pub fn apply_horizontal_pan(
    current_zoom: f64,
    current_pan_norm: f64,
    delta_px: f64,
    viewport_width_px: f64,
) -> f64 {
    if viewport_width_px <= 1.0 || current_zoom <= 1.0 {
        return 0.0;
    }
    let visible = 1.0 / current_zoom;
    let max_pan = (1.0 - visible).max(0.0);
    let delta_norm = -(delta_px / viewport_width_px) * visible;
    (current_pan_norm + delta_norm).clamp(0.0, max_pan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinch_tracker_computes_scale_and_pan() {
        let mut t = PinchPanTracker::new();
        t.pointer_down(1, 100.0, 50.0);
        assert_eq!(
            t.pointer_move(1, 120.0, 55.0),
            Some(GestureDelta::Pan { dx: 20.0, dy: 5.0 })
        );

        // Add second finger 100px away
        t.pointer_down(2, 220.0, 55.0);
        // Move second finger to 200px away -> 2x zoom
        let delta = t.pointer_move(2, 320.0, 55.0).expect("pinch delta");
        match delta {
            GestureDelta::Pinch {
                scale_ratio,
                center_x,
                pan_dx,
                ..
            } => {
                assert!((scale_ratio - 2.0).abs() < 1e-6);
                assert!((center_x - 220.0).abs() < 1e-6);
                assert!((pan_dx - 50.0).abs() < 1e-6);
            }
            _ => panic!("expected pinch"),
        }
    }

    #[test]
    fn anchored_zoom_keeps_world_point_stationary() {
        let (z, pan) = apply_anchored_zoom(1.0, 0.0, 2.0, 0.5, 1.0, 32.0);
        assert!((z - 2.0).abs() < 1e-9);
        assert!((pan - 0.25).abs() < 1e-9);
    }

    #[test]
    fn horizontal_pan_clamps_to_bounds() {
        let pan = apply_horizontal_pan(2.0, 0.25, -200.0, 400.0);
        assert!((pan - 0.5).abs() < 1e-9);
        let pan_clamped = apply_horizontal_pan(2.0, 0.4, -400.0, 400.0);
        assert!((pan_clamped - 0.5).abs() < 1e-9);
    }
}
