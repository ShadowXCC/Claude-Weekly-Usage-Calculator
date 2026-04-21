/// Renders the tray icon as a PNG byte vector.
///
/// Three modes are supported:
///   • Ring   — the classic circle outline with a "U" in the center; the
///              clockwise-from-12 arc fills in proportion to `fill_fraction`.
///   • Number — a transparent background with a two-digit seven-segment
///              readout of `value` (percent).
///   • Emoji  — a solid colored dot (green/yellow/red) indicating budget
///              health. The platform's tray title label carries the actual
///              "🟢42%" string (set separately by the caller). The dot doubles
///              as the Windows fallback where title labels are not supported.
///
/// Colors derive from the `accent` RGBA (for Ring and Number); the Emoji dot
/// uses fixed green/yellow/red so it communicates at a glance regardless of
/// the user's accent choice.
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayMode {
    Ring,
    Number,
    Emoji,
}

impl TrayMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "number" => TrayMode::Number,
            "emoji" => TrayMode::Emoji,
            _ => TrayMode::Ring,
        }
    }
}

/// Parse an `#RRGGBB` or `RRGGBB` hex string into an RGBA tuple (alpha 255).
/// Returns Claude orange on any parse error.
pub fn parse_hex_rgba(hex: &str) -> (u8, u8, u8, u8) {
    let s = hex.trim_start_matches('#');
    if s.len() != 6 {
        return (0xDA, 0x77, 0x56, 0xFF);
    }
    u32::from_str_radix(s, 16)
        .ok()
        .map(|n| (((n >> 16) & 0xFF) as u8, ((n >> 8) & 0xFF) as u8, (n & 0xFF) as u8, 0xFF))
        .unwrap_or((0xDA, 0x77, 0x56, 0xFF))
}

pub fn render_tray_icon(
    fill_fraction: f64,
    size: u32,
    mode: TrayMode,
    accent: (u8, u8, u8, u8),
    value: u32,
    _inverse: bool, // reserved; value is already pre-flipped by caller if needed
) -> Vec<u8> {
    let size_f = size as f32;
    let mut pixmap = Pixmap::new(size, size).unwrap();
    pixmap.fill(Color::TRANSPARENT);

    match mode {
        TrayMode::Ring => draw_ring(&mut pixmap, size_f, fill_fraction, accent),
        TrayMode::Number => draw_number(&mut pixmap, size_f, value, accent),
        TrayMode::Emoji => draw_emoji_dot(&mut pixmap, size_f, fill_fraction),
    }

    pixmap.encode_png().unwrap_or_default()
}

// ── Ring mode ────────────────────────────────────────────────────────────────

fn draw_ring(pixmap: &mut Pixmap, size_f: f32, fill_fraction: f64, accent: (u8, u8, u8, u8)) {
    let cx = size_f / 2.0;
    let cy = size_f / 2.0;
    let ring_outer = size_f * 0.46;
    let ring_inner = size_f * 0.30;

    let accent_color = Color::from_rgba8(accent.0, accent.1, accent.2, accent.3);
    let unfilled = Color::from_rgba8(180, 170, 160, 200); // Muted warm gray

    draw_arc_ring(pixmap, cx, cy, ring_outer, ring_inner, 0.0, 1.0, unfilled);
    if fill_fraction > 0.001 {
        let frac = fill_fraction.clamp(0.0, 1.0) as f32;
        draw_arc_ring(pixmap, cx, cy, ring_outer, ring_inner, 0.0, frac, accent_color);
    }
    draw_u_letter(pixmap, cx, cy, size_f * 0.22, accent_color);
}

// ── Emoji mode (color-coded dot) ─────────────────────────────────────────────

fn draw_emoji_dot(pixmap: &mut Pixmap, size_f: f32, fill_fraction: f64) {
    // Map the used % to a traffic-light tier. fill_fraction is 0..1 = 0..100%.
    let pct = fill_fraction * 100.0;
    let color = if pct < 50.0 {
        Color::from_rgba8(0x4C, 0xAF, 0x50, 0xFF) // green
    } else if pct < 80.0 {
        Color::from_rgba8(0xFF, 0xC1, 0x07, 0xFF) // amber
    } else {
        Color::from_rgba8(0xE5, 0x3E, 0x3E, 0xFF) // red
    };

    let cx = size_f / 2.0;
    let cy = size_f / 2.0;
    let r = size_f * 0.36;

    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    let path = match pb.finish() {
        Some(p) => p,
        None => return,
    };

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

// ── Number mode (seven-segment two-digit readout) ────────────────────────────

/// Segments per digit, bit order: [top, top-right, bottom-right, bottom,
/// bottom-left, top-left, middle]. true = lit.
const DIGITS: [[bool; 7]; 10] = [
    [true,  true,  true,  true,  true,  true,  false], // 0
    [false, true,  true,  false, false, false, false], // 1
    [true,  true,  false, true,  true,  false, true ], // 2
    [true,  true,  true,  true,  false, false, true ], // 3
    [false, true,  true,  false, false, true,  true ], // 4
    [true,  false, true,  true,  false, true,  true ], // 5
    [true,  false, true,  true,  true,  true,  true ], // 6
    [true,  true,  true,  false, false, false, false], // 7
    [true,  true,  true,  true,  true,  true,  true ], // 8
    [true,  true,  true,  true,  false, true,  true ], // 9
];

fn draw_number(pixmap: &mut Pixmap, size_f: f32, value: u32, accent: (u8, u8, u8, u8)) {
    let v = value.min(99);
    let tens = (v / 10) as usize;
    let ones = (v % 10) as usize;

    // Digit box dimensions (scaled relative to icon size).
    let digit_w = size_f * 0.34;
    let digit_h = size_f * 0.66;
    let gap = size_f * 0.08;
    let total_w = digit_w * 2.0 + gap;
    let x0 = (size_f - total_w) / 2.0;
    let y0 = (size_f - digit_h) / 2.0;

    let color = Color::from_rgba8(accent.0, accent.1, accent.2, accent.3);

    // When the number is < 10, render only the ones digit and hide the tens slot.
    // (Leaving just one big digit reads better than "07" on a 22-pixel tray.)
    if v < 10 {
        let single_w = digit_w * 1.4; // slightly larger single digit
        let sx = (size_f - single_w) / 2.0;
        draw_seven_seg(pixmap, sx, y0, single_w, digit_h, ones, color);
    } else {
        draw_seven_seg(pixmap, x0, y0, digit_w, digit_h, tens, color);
        draw_seven_seg(pixmap, x0 + digit_w + gap, y0, digit_w, digit_h, ones, color);
    }
}

fn draw_seven_seg(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    digit: usize,
    color: Color,
) {
    let on = DIGITS[digit];
    let thick = w.min(h) * 0.18; // segment thickness
    let half_h = (h - 3.0 * thick) * 0.5; // vertical segment length (half of inner height)
    let seg_w = w - 2.0 * thick;            // horizontal segment length

    // [top, top-right, bottom-right, bottom, bottom-left, top-left, middle]
    let segs = [
        (on[0], Rect::from_xywh(x + thick, y, seg_w, thick)),                                    // top
        (on[1], Rect::from_xywh(x + w - thick, y + thick, thick, half_h)),                       // top-right
        (on[2], Rect::from_xywh(x + w - thick, y + thick + half_h + thick, thick, half_h)),      // bottom-right
        (on[3], Rect::from_xywh(x + thick, y + h - thick, seg_w, thick)),                        // bottom
        (on[4], Rect::from_xywh(x, y + thick + half_h + thick, thick, half_h)),                  // bottom-left
        (on[5], Rect::from_xywh(x, y + thick, thick, half_h)),                                   // top-left
        (on[6], Rect::from_xywh(x + thick, y + thick + half_h, seg_w, thick)),                   // middle
    ];

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;

    for (lit, rect_opt) in segs {
        if !lit {
            continue;
        }
        let rect = match rect_opt {
            Some(r) => r,
            None => continue,
        };
        let mut pb = PathBuilder::new();
        pb.push_rect(rect);
        if let Some(path) = pb.finish() {
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
}

// ── Primitives (shared) ──────────────────────────────────────────────────────

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
    let p0 = (cx - outer_bowl_r, bowl_cy);
    pb.move_to(p0.0, p0.1);
    for i in 1..=segments {
        let t = PI * (i as f32 / segments as f32);
        let x = cx - outer_bowl_r * t.cos();
        let y = bowl_cy + outer_bowl_r * t.sin();
        pb.line_to(x, y);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_rgba() {
        assert_eq!(parse_hex_rgba("#DA7756"), (0xDA, 0x77, 0x56, 0xFF));
        assert_eq!(parse_hex_rgba("DA7756"), (0xDA, 0x77, 0x56, 0xFF));
        assert_eq!(parse_hex_rgba("#00FF00"), (0x00, 0xFF, 0x00, 0xFF));
        // Invalid → default orange fallback
        assert_eq!(parse_hex_rgba("#GGG"), (0xDA, 0x77, 0x56, 0xFF));
        assert_eq!(parse_hex_rgba(""), (0xDA, 0x77, 0x56, 0xFF));
    }

    #[test]
    fn test_number_mode_nonempty() {
        let png = render_tray_icon(0.42, 64, TrayMode::Number, (0xDA, 0x77, 0x56, 0xFF), 42, false);
        assert!(!png.is_empty());
    }

    #[test]
    fn test_ring_mode_nonempty() {
        let png = render_tray_icon(0.5, 64, TrayMode::Ring, (0xDA, 0x77, 0x56, 0xFF), 50, false);
        assert!(!png.is_empty());
    }

    #[test]
    fn test_emoji_mode_nonempty() {
        for pct in [10.0_f64, 60.0, 95.0] {
            let png = render_tray_icon(pct / 100.0, 64, TrayMode::Emoji, (0, 0, 0, 0xFF), pct as u32, false);
            assert!(!png.is_empty());
        }
    }
}
