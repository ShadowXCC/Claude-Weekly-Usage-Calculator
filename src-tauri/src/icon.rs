/// Renders the tray icon as a PNG byte vector.
///
/// The icon is a circle outline with a "U" in the center.
/// The circle arc (clockwise from 12 o'clock) is filled in proportion to `fill_fraction` (0.0–1.0).
/// Filled portion uses Claude orange; unfilled portion and the "U" use a light neutral color.
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

pub fn render_tray_icon(fill_fraction: f64, size: u32) -> Vec<u8> {
    let size_f = size as f32;
    let mut pixmap = Pixmap::new(size, size).unwrap();

    // Transparent background
    pixmap.fill(Color::TRANSPARENT);

    let cx = size_f / 2.0;
    let cy = size_f / 2.0;
    let ring_outer = size_f * 0.46;
    let ring_inner = size_f * 0.30;

    // Colors
    let orange = Color::from_rgba8(218, 119, 86, 255); // #DA7756 Claude orange
    let unfilled = Color::from_rgba8(180, 170, 160, 200); // Muted warm gray

    // Draw unfilled arc background (full circle, muted)
    draw_arc_ring(&mut pixmap, cx, cy, ring_outer, ring_inner, 0.0, 1.0, unfilled);

    // Draw filled arc (clockwise from top)
    if fill_fraction > 0.001 {
        let frac = fill_fraction.clamp(0.0, 1.0) as f32;
        draw_arc_ring(&mut pixmap, cx, cy, ring_outer, ring_inner, 0.0, frac, orange);
    }

    // Draw "U" letter in center
    draw_u_letter(&mut pixmap, cx, cy, size_f * 0.22, orange);

    // Encode to PNG
    pixmap.encode_png().unwrap_or_default()
}

/// Draw a ring arc segment. `start_frac` and `end_frac` are 0.0–1.0 fractions of the full circle,
/// with 0.0 at 12 o'clock, advancing clockwise.
fn draw_arc_ring(
    pixmap: &mut Pixmap,
    cx: f32,
    cy: f32,
    outer_r: f32,
    inner_r: f32,
    start_frac: f32,
    end_frac: f32,
    color: Color,
) {
    use std::f32::consts::{PI, TAU};

    let segments = 128usize;
    let start_angle = start_frac * TAU - PI / 2.0; // offset so 0 = top
    let end_angle = end_frac * TAU - PI / 2.0;

    let total_angle = end_frac - start_frac;
    let steps = ((total_angle * segments as f32) as usize).max(2);

    let mut pb = PathBuilder::new();

    // Outer arc points
    let outer_pts: Vec<(f32, f32)> = (0..=steps)
        .map(|i| {
            let t = start_angle + (end_angle - start_angle) * (i as f32 / steps as f32);
            (cx + outer_r * t.cos(), cy + outer_r * t.sin())
        })
        .collect();

    // Inner arc points (reversed for closing)
    let inner_pts: Vec<(f32, f32)> = (0..=steps)
        .rev()
        .map(|i| {
            let t = start_angle + (end_angle - start_angle) * (i as f32 / steps as f32);
            (cx + inner_r * t.cos(), cy + inner_r * t.sin())
        })
        .collect();

    if outer_pts.is_empty() {
        return;
    }

    pb.move_to(outer_pts[0].0, outer_pts[0].1);
    for pt in &outer_pts[1..] {
        pb.line_to(pt.0, pt.1);
    }
    for pt in &inner_pts {
        pb.line_to(pt.0, pt.1);
    }
    pb.close();

    let path = pb.finish().unwrap();

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;

    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

/// Draw a simple "U" shape using filled rectangles/arcs.
fn draw_u_letter(pixmap: &mut Pixmap, cx: f32, cy: f32, scale: f32, color: Color) {
    use std::f32::consts::PI;

    let stroke_w = scale * 0.30;
    let half_w = scale * 0.55;
    let arm_height = scale * 0.70;

    // Center the U: top of arms at cy - arm_height, bowl center at cy - arm_height + bowl_r + stroke_w/2
    let top_y = cy - scale * 0.85;
    let bowl_cy = top_y + arm_height;

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;

    // Left arm
    let left_x = cx - half_w;
    if let Some(rect) = Rect::from_xywh(left_x, top_y, stroke_w, arm_height) {
        let mut pb = PathBuilder::new();
        pb.push_rect(rect);
        if let Some(p) = pb.finish() {
            pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    // Right arm
    let right_x = cx + half_w - stroke_w;
    if let Some(rect) = Rect::from_xywh(right_x, top_y, stroke_w, arm_height) {
        let mut pb = PathBuilder::new();
        pb.push_rect(rect);
        if let Some(p) = pb.finish() {
            pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    // Bowl (bottom semicircle) — filled ring arc
    let segments = 32usize;
    let outer_bowl_r = half_w;
    let inner_bowl_r = half_w - stroke_w;

    let mut pb = PathBuilder::new();
    // outer semicircle (left to right, going downward)
    let p0 = (cx - outer_bowl_r, bowl_cy);
    pb.move_to(p0.0, p0.1);
    for i in 1..=segments {
        let t = PI * (i as f32 / segments as f32);
        let x = cx - outer_bowl_r * t.cos();
        let y = bowl_cy + outer_bowl_r * t.sin();
        pb.line_to(x, y);
    }
    // inner semicircle (right to left)
    for i in (0..=segments).rev() {
        let t = PI * (i as f32 / segments as f32);
        let x = cx - inner_bowl_r * t.cos();
        let y = bowl_cy + inner_bowl_r * t.sin();
        pb.line_to(x, y);
    }
    pb.close();

    if let Some(p) = pb.finish() {
        pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
    }
}
