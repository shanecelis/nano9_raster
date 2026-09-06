//! Shared autoplay scene used by the WASM demo and the GIF recorder.

use nano9_raster::{
    Circle, CircleAa, Ellipse, EllipseAa, Fill, Inclusive, Line, LineAa, Plot, Point, QuadBezier,
    QuadBezierAa, RoundRect, RoundRectAa, ThickLine, ThickLineAa, ThickLineFillAa,
};

#[cfg(feature = "murphy")]
use nano9_raster::murphy::ThickLineFill;
#[cfg(not(feature = "murphy"))]
use nano9_raster::ThickLineFill;

pub const WIDTH: u32 = 64;
pub const HEIGHT: u32 = 48;
pub const BG: [u8; 4] = [0x11, 0x11, 0x11, 0xff];
pub const REVEAL_FRAMES: usize = 45;
pub const HOLD_FRAMES: u32 = 60;

const DIGITS: [u16; 10] = [
    0b111_101_101_101_111,
    0b010_110_010_010_111,
    0b111_001_111_100_111,
    0b111_001_111_001_111,
    0b101_101_111_001_001,
    0b111_100_111_001_111,
    0b111_100_111_101_111,
    0b111_001_001_001_001,
    0b111_101_111_101_111,
    0b111_101_111_001_111,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Line,
    Circle,
    Ellipse,
    QuadBezier,
    ThickLine,
    RoundRect,
}

impl Kind {
    pub fn index(self) -> usize {
        match self {
            Kind::Line => 0,
            Kind::Circle => 1,
            Kind::Ellipse => 2,
            Kind::QuadBezier => 3,
            Kind::ThickLine => 4,
            Kind::RoundRect => 5,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Kind::Line => Kind::Circle,
            Kind::Circle => Kind::Ellipse,
            Kind::Ellipse => Kind::QuadBezier,
            Kind::QuadBezier => Kind::ThickLine,
            Kind::ThickLine => Kind::RoundRect,
            Kind::RoundRect => Kind::Line,
        }
    }

    pub fn is_bezier(self) -> bool {
        self == Kind::QuadBezier
    }

    pub fn has_control_point(self) -> bool {
        matches!(self, Kind::QuadBezier | Kind::ThickLine | Kind::RoundRect)
    }

    pub fn supports_aa(self) -> bool {
        matches!(
            self,
            Kind::Line
                | Kind::Circle
                | Kind::Ellipse
                | Kind::QuadBezier
                | Kind::ThickLine
                | Kind::RoundRect
        )
    }

    pub fn supports_fill(self) -> bool {
        matches!(
            self,
            Kind::Circle | Kind::Ellipse | Kind::ThickLine | Kind::RoundRect
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Shape,
    AntiAlias,
    Fill,
    Play,
}

const AUTO_STATES: [(Kind, bool, bool); 20] = [
    (Kind::Line, false, false),
    (Kind::Line, true, false),
    (Kind::Circle, false, false),
    (Kind::Circle, true, false),
    (Kind::Circle, false, true),
    (Kind::Circle, true, true),
    (Kind::Ellipse, false, false),
    (Kind::Ellipse, true, false),
    (Kind::Ellipse, false, true),
    (Kind::Ellipse, true, true),
    (Kind::QuadBezier, false, false),
    (Kind::QuadBezier, true, false),
    (Kind::ThickLine, false, false),
    (Kind::ThickLine, true, false),
    (Kind::ThickLine, false, true),
    (Kind::ThickLine, true, true),
    (Kind::RoundRect, false, false),
    (Kind::RoundRect, true, false),
    (Kind::RoundRect, false, true),
    (Kind::RoundRect, true, true),
];

pub struct Scene {
    pub buf: Vec<u8>,
    pub start: Point,
    pub end: Point,
    pub control: Point,
    pub kind: Kind,
    pub anti_alias: bool,
    pub fill: bool,
    pub pixels: Vec<(Point, u8)>,
    pub tour: u32,
    auto_state: usize,
}

impl Scene {
    pub fn new() -> Self {
        let mut scene = Scene {
            buf: vec![0; (WIDTH * HEIGHT * 4) as usize],
            start: (4, 8),
            end: (59, 39),
            control: (0, 0),
            kind: Kind::Line,
            anti_alias: false,
            fill: false,
            pixels: Vec::new(),
            tour: 0,
            auto_state: 0,
        };
        scene.load_shape();
        scene.clear();
        scene
    }

    pub fn geometry(kind: Kind, tour: u32) -> (Point, Point) {
        let i = (tour as usize) % 3;
        match kind {
            Kind::Line => [
                ((8, 14), (58, 38)),
                ((8, 38), (58, 12)),
                ((12, 24), (56, 24)),
            ][i],
            Kind::Circle => [
                ((32, 26), (46, 26)),
                ((28, 28), (40, 28)),
                ((36, 24), (48, 24)),
            ][i],
            Kind::Ellipse => [
                ((14, 12), (54, 40)),
                ((18, 16), (50, 36)),
                ((10, 20), (58, 32)),
            ][i],
            Kind::QuadBezier => [
                ((8, 36), (58, 36)),
                ((10, 12), (54, 40)),
                ((8, 40), (58, 14)),
            ][i],
            Kind::ThickLine => [
                ((10, 16), (54, 36)),
                ((10, 36), (54, 14)),
                ((12, 22), (56, 28)),
            ][i],
            Kind::RoundRect => [
                ((8, 12), (56, 38)),
                ((12, 10), (50, 36)),
                ((10, 16), (58, 32)),
            ][i],
        }
    }

    pub fn load_shape(&mut self) {
        let (start, end) = Self::geometry(self.kind, self.tour);
        self.start = start;
        self.end = end;
        self.reset_control();
        self.pixels = self.collect_pixels();
    }

    pub fn collect_pixels(&self) -> Vec<(Point, u8)> {
        let start = self.start;
        let end = self.end;
        match self.kind {
            Kind::Line if self.anti_alias => {
                LineAa::new(start, end).filter(|(_, c)| *c > 0).collect()
            }
            Kind::Line => Line::new(start, end).map(|p| (p, 255)).collect(),
            Kind::Circle if self.anti_alias && self.fill => {
                Self::expand_plots(CircleAa::new(start, Self::radius(start, end)).fill())
            }
            Kind::Circle if self.anti_alias => CircleAa::new(start, Self::radius(start, end))
                .filter(|(_, c)| *c > 0)
                .collect(),
            Kind::Circle if self.fill => Circle::new(start, Self::radius(start, end))
                .fill()
                .flat_map(|h| (h.x0..=h.x1).map(move |x| ((x, h.y), 255)))
                .collect(),
            Kind::Circle => Circle::new(start, Self::radius(start, end))
                .map(|p| (p, 255))
                .collect(),
            Kind::Ellipse if self.anti_alias && self.fill => {
                Self::expand_plots(EllipseAa::from_rect(start, end).fill())
            }
            Kind::Ellipse if self.anti_alias => EllipseAa::from_rect(start, end)
                .filter(|(_, c)| *c > 0)
                .collect(),
            Kind::Ellipse if self.fill => Ellipse::from_rect(start, end)
                .fill()
                .flat_map(|h| (h.x0..=h.x1).map(move |x| ((x, h.y), 255)))
                .collect(),
            Kind::Ellipse => Ellipse::from_rect(start, end).map(|p| (p, 255)).collect(),
            Kind::QuadBezier if self.anti_alias => QuadBezierAa::new(start, self.control, end)
                .filter(|(_, c)| *c > 0)
                .collect(),
            Kind::QuadBezier => QuadBezier::new(start, self.control, end)
                .map(|p| (p, 255))
                .collect(),
            Kind::ThickLine if self.anti_alias && self.fill => {
                ThickLineFillAa::new(start, end, self.thick_line_width())
                    .filter(|(_, c)| *c > 0)
                    .collect()
            }
            Kind::ThickLine if self.anti_alias => {
                ThickLineAa::new(start, end, self.thick_line_width())
                    .filter(|(_, c)| *c > 0)
                    .collect()
            }
            Kind::ThickLine if self.fill => ThickLineFill::new(start, end, self.thick_line_width())
                .map(|p| (p, 255))
                .collect(),
            Kind::ThickLine => ThickLine::new(start, end, self.thick_line_width())
                .map(|p| (p, 255))
                .collect(),
            Kind::RoundRect if self.anti_alias && self.fill => {
                Self::expand_plots(RoundRectAa::new(start, end, self.round_rect_radius()).fill())
            }
            Kind::RoundRect if self.anti_alias => {
                RoundRectAa::new(start, end, self.round_rect_radius())
                    .filter(|(_, c)| *c > 0)
                    .collect()
            }
            Kind::RoundRect if self.fill => RoundRect::new(start, end, self.round_rect_radius())
                .fill()
                .flat_map(|h| (h.x0..=h.x1).map(move |x| ((x, h.y), 255)))
                .collect(),
            Kind::RoundRect => RoundRect::new(start, end, self.round_rect_radius())
                .map(|p| (p, 255))
                .collect(),
        }
    }

    fn expand_plots(plots: impl Iterator<Item = Plot>) -> Vec<(Point, u8)> {
        let mut pixels = Vec::new();
        for plot in plots {
            match plot {
                Plot::Span(h) => pixels.extend((h.x0..=h.x1).map(|x| ((x, h.y), 255))),
                Plot::Point((p, c)) => {
                    if c > 0 {
                        pixels.push((p, c));
                    }
                }
            }
        }
        pixels
    }

    pub fn control_point(start: Point, end: Point) -> Point {
        let (x0, y0) = start;
        let (x1, y1) = end;
        ((x0 + x1) / 2 - (y1 - y0) / 3, (y0 + y1) / 2 + (x1 - x0) / 3)
    }

    fn round_rect_bounds(start: Point, end: Point) -> (isize, isize, isize, isize) {
        (
            start.0.min(end.0),
            start.1.min(end.1),
            start.0.max(end.0),
            start.1.max(end.1),
        )
    }

    fn default_round_rect_radius(start: Point, end: Point) -> isize {
        let (x0, y0, x1, y1) = Self::round_rect_bounds(start, end);
        ((x1 - x0).min(y1 - y0) / 4).max(0)
    }

    fn round_rect_control(start: Point, end: Point) -> Point {
        let (x0, y0, _, _) = Self::round_rect_bounds(start, end);
        let r = Self::default_round_rect_radius(start, end);
        (x0 + r, y0 + r)
    }

    pub fn round_rect_radius(&self) -> isize {
        let (x0, y0, _, _) = Self::round_rect_bounds(self.start, self.end);
        (self.control.0 - x0)
            .max(0)
            .min((self.control.1 - y0).max(0))
    }

    fn chord_len(start: Point, end: Point) -> f64 {
        let dx = (end.0 - start.0) as f64;
        let dy = (end.1 - start.1) as f64;
        (dx * dx + dy * dy).sqrt()
    }

    fn thick_line_control(start: Point, end: Point) -> Point {
        let mid = ((start.0 + end.0) / 2, (start.1 + end.1) / 2);
        let dx = (end.0 - start.0) as f64;
        let dy = (end.1 - start.1) as f64;
        let len = Self::chord_len(start, end);
        if len < 1.0 {
            return (mid.0, mid.1 - 3);
        }
        let ox = (-dy * 3.0 / len).round() as isize;
        let oy = (dx * 3.0 / len).round() as isize;
        if ox == 0 && oy == 0 {
            (mid.0, mid.1 - 3)
        } else {
            (mid.0 + ox, mid.1 + oy)
        }
    }

    pub fn thick_line_width(&self) -> f32 {
        let len = Self::chord_len(self.start, self.end);
        let dist = if len < 1.0 {
            let dx = (self.control.0 - self.start.0) as f64;
            let dy = (self.control.1 - self.start.1) as f64;
            (dx * dx + dy * dy).sqrt()
        } else {
            let dx = (self.end.0 - self.start.0) as f64;
            let dy = (self.end.1 - self.start.1) as f64;
            ((self.control.0 - self.start.0) as f64 * dy
                - (self.control.1 - self.start.1) as f64 * dx)
                .abs()
                / len
        };
        dist.max(1.0) as f32
    }

    pub fn reset_control(&mut self) {
        self.control = match self.kind {
            Kind::RoundRect => Self::round_rect_control(self.start, self.end),
            Kind::ThickLine => Self::thick_line_control(self.start, self.end),
            _ => Self::control_point(self.start, self.end),
        };
    }

    pub fn handle_point(&self) -> Point {
        match self.kind {
            Kind::RoundRect => {
                let (x0, y0, _, _) = Self::round_rect_bounds(self.start, self.end);
                let r = self.round_rect_radius();
                (x0 + r, y0 + r)
            }
            Kind::ThickLine => {
                let mid = (
                    (self.start.0 + self.end.0) / 2,
                    (self.start.1 + self.end.1) / 2,
                );
                let dx = (self.end.0 - self.start.0) as f64;
                let dy = (self.end.1 - self.start.1) as f64;
                let len = Self::chord_len(self.start, self.end);
                let w = f64::from(self.thick_line_width());
                if len < 1.0 {
                    return (mid.0, mid.1 - w.round() as isize);
                }
                let cross = (self.control.0 - self.start.0) as f64 * dy
                    - (self.control.1 - self.start.1) as f64 * dx;
                let sign = if cross > 0.0 { -1.0 } else { 1.0 };
                let ox = (-dy * sign * w / len).round() as isize;
                let oy = (dx * sign * w / len).round() as isize;
                (mid.0 + ox, mid.1 + oy)
            }
            _ => self.control,
        }
    }

    pub fn near_control(&self, p: Point) -> bool {
        let (x, y) = self.handle_point();
        (p.0 - x).abs() <= 2 && (p.1 - y).abs() <= 2
    }

    pub fn plot_control(&mut self) {
        if !self.kind.has_control_point() {
            return;
        }
        let (x, y) = self.handle_point();
        for p in [(x, y), (x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            self.plot(p, 255, 140, 0);
        }
    }

    pub fn radius(start: Point, end: Point) -> isize {
        (end.0 - start.0).abs().max((end.1 - start.1).abs()).max(1)
    }

    pub fn clear(&mut self) {
        for px in self.buf.chunks_exact_mut(4) {
            px.copy_from_slice(&BG);
        }
    }

    pub fn plot(&mut self, (x, y): Point, r: u8, g: u8, b: u8) {
        if x < 0 || y < 0 || x >= WIDTH as isize || y >= HEIGHT as isize {
            return;
        }
        let i = ((y as u32 * WIDTH + x as u32) * 4) as usize;
        self.buf[i] = r;
        self.buf[i + 1] = g;
        self.buf[i + 2] = b;
        self.buf[i + 3] = 255;
    }

    pub fn stamp_digit(&mut self) {
        // Controls occupy a fixed strip over the top of the drawing.
        for y in 0..7 {
            for x in 0..24 {
                self.plot((x, y), BG[0], BG[1], BG[2]);
            }
        }

        let bits = DIGITS[self.kind.index()];
        for row in 0..5 {
            for col in 0..3 {
                let bit = row * 3 + col;
                if (bits >> (14 - bit)) & 1 == 1 {
                    self.plot((1 + col, 1 + row), 255, 255, 255);
                }
            }
        }

        let aa_color = if self.kind.supports_aa() { 255 } else { 0x55 };
        let a_segments = [((8, 6), (10, 1)), ((10, 1), (13, 6)), ((9, 4), (12, 4))];
        if self.anti_alias {
            for (start, end) in a_segments {
                let points: Vec<_> = LineAa::new(start, end).collect();
                for (point, alpha) in points {
                    let color = (u16::from(alpha) * u16::from(aa_color) / 255) as u8;
                    self.plot(point, color, color, color);
                }
            }
        } else {
            for (start, end) in a_segments {
                let points: Vec<_> = Line::new(start, end).inclusive().collect();
                for point in points {
                    self.plot(point, aa_color, aa_color, aa_color);
                }
            }
        }

        let fill_color = if self.kind.supports_fill() { 255 } else { 0x55 };
        if self.fill {
            let spans: Vec<_> = Circle::new((18, 3), 2).fill().collect();
            for span in spans {
                for x in span.x0..=span.x1 {
                    self.plot((x, span.y), fill_color, fill_color, fill_color);
                }
            }
        } else {
            let points: Vec<_> = Circle::new((18, 3), 2).collect();
            for point in points {
                self.plot(point, fill_color, fill_color, fill_color);
            }
        }
    }

    /// Play triangle when paused; pause bars while autoplay is running.
    pub fn stamp_play(&mut self, playing: bool) {
        for y in 0..7 {
            for x in 24..31 {
                self.plot((x, y), BG[0], BG[1], BG[2]);
            }
        }
        if playing {
            for y in 1..6 {
                self.plot((25, y), 255, 255, 255);
                self.plot((27, y), 255, 255, 255);
            }
        } else {
            for (i, w) in [1isize, 2, 3, 2, 1].iter().enumerate() {
                for dx in 0..*w {
                    self.plot((25 + dx, 1 + i as isize), 255, 255, 255);
                }
            }
        }
    }

    pub fn reveal_count(&self) -> usize {
        let n = self.pixels.len();
        if n == 0 {
            1
        } else {
            n.div_ceil(REVEAL_FRAMES)
        }
    }

    pub fn next_kind(&mut self) {
        let next = self.kind.next();
        if next == Kind::Line {
            self.tour = self.tour.wrapping_add(1);
        }
        self.kind = next;
    }

    pub fn control_at((x, y): Point) -> Option<Control> {
        if !(0..7).contains(&y) {
            return None;
        }
        match x {
            0..=5 => Some(Control::Shape),
            7..=13 => Some(Control::AntiAlias),
            15..=22 => Some(Control::Fill),
            24..=29 => Some(Control::Play),
            _ => None,
        }
    }

    pub fn activate_control(&mut self, control: Control) {
        match control {
            Control::Shape => {
                self.next_kind();
                if self.kind.has_control_point() {
                    self.reset_control();
                }
            }
            Control::AntiAlias if self.kind.supports_aa() => {
                self.anti_alias = !self.anti_alias;
            }
            Control::Fill if self.kind.supports_fill() => {
                self.fill = !self.fill;
            }
            Control::AntiAlias | Control::Fill | Control::Play => return,
        }
        self.pixels = self.collect_pixels();
    }

    pub fn sync_auto_state(&mut self) {
        if let Some(i) = AUTO_STATES.iter().position(|&(kind, aa, fill)| {
            kind == self.kind && aa == self.anti_alias && fill == self.fill
        }) {
            self.auto_state = i;
        }
    }

    pub fn advance_kind(&mut self) {
        self.auto_state = (self.auto_state + 1) % AUTO_STATES.len();
        if self.auto_state == 0 {
            self.tour = self.tour.wrapping_add(1);
        }
        (self.kind, self.anti_alias, self.fill) = AUTO_STATES[self.auto_state];
        self.load_shape();
        self.clear();
    }

    pub fn indexed(&self) -> Vec<u8> {
        self.buf
            .chunks_exact(4)
            .map(|px| if px == BG { 0 } else { px[0] })
            .collect()
    }
}

/// One autoplay tour through every applicable shape/AA/fill combination. Each
/// entry is a unique canvas and a GIF delay
/// in hundredths of a second.
pub fn autoplay_frames() -> Vec<(Vec<u8>, u16)> {
    let mut scene = Scene::new();
    let mut frames = Vec::new();
    for state_i in 0..AUTO_STATES.len() {
        let mut step = 0usize;
        loop {
            if scene.pixels.is_empty() {
                scene.stamp_digit();
                frames.push((scene.indexed(), hold_delay_cs()));
                break;
            }
            if step == 0 {
                scene.clear();
            }
            let n = scene.reveal_count();
            let end = (step + n).min(scene.pixels.len());
            for i in step..end {
                let (p, c) = scene.pixels[i];
                scene.plot(p, c, c, c);
            }
            step = end;
            scene.plot_control();
            scene.stamp_digit();
            let done = step >= scene.pixels.len();
            let delay = if done {
                hold_delay_cs()
            } else {
                reveal_delay_cs()
            };
            frames.push((scene.indexed(), delay));
            if done {
                break;
            }
        }
        if state_i + 1 < AUTO_STATES.len() {
            scene.advance_kind();
        }
    }
    coalesce(frames)
}

fn reveal_delay_cs() -> u16 {
    // 1/60 s ≈ 1.67 cs; GIF delay 2 = 20 ms.
    2
}

fn hold_delay_cs() -> u16 {
    // HOLD_FRAMES / 60 s → hundredths.
    (HOLD_FRAMES as u16 * 100) / 60
}

fn coalesce(frames: Vec<(Vec<u8>, u16)>) -> Vec<(Vec<u8>, u16)> {
    let mut out: Vec<(Vec<u8>, u16)> = Vec::new();
    for (buf, delay) in frames {
        if let Some((last, last_delay)) = out.last_mut() {
            if *last == buf {
                *last_delay = last_delay.saturating_add(delay);
                continue;
            }
        }
        out.push((buf, delay));
    }
    out
}

pub fn palette() -> Vec<u8> {
    let mut pal = Vec::with_capacity(256 * 3);
    pal.extend_from_slice(&BG[..3]);
    for i in 1..=255u8 {
        pal.extend_from_slice(&[i, i, i]);
    }
    pal
}

pub fn scale_indexed(src: &[u8], scale: u32) -> Vec<u8> {
    let w = WIDTH * scale;
    let h = HEIGHT * scale;
    let mut out = vec![0u8; (w * h) as usize];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let c = src[(y * WIDTH + x) as usize];
            let x0 = x * scale;
            let y0 = y * scale;
            for dy in 0..scale {
                let row = ((y0 + dy) * w + x0) as usize;
                out[row..row + scale as usize].fill(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Control, Kind, Scene, BG};

    #[test]
    fn base_shape_capabilities_are_explicit() {
        assert!(Kind::Line.supports_aa());
        assert!(Kind::Circle.supports_aa());
        assert!(Kind::Circle.supports_fill());
        assert!(Kind::Ellipse.supports_aa());
        assert!(Kind::Ellipse.supports_fill());
        assert!(Kind::QuadBezier.supports_aa());
        assert!(!Kind::Line.supports_fill());
        assert!(Kind::ThickLine.supports_aa());
        assert!(Kind::ThickLine.supports_fill());
        assert!(Kind::RoundRect.supports_aa());
        assert!(Kind::RoundRect.supports_fill());
    }

    #[test]
    fn controls_have_distinct_hit_targets() {
        assert_eq!(Scene::control_at((2, 3)), Some(Control::Shape));
        assert_eq!(Scene::control_at((10, 3)), Some(Control::AntiAlias));
        assert_eq!(Scene::control_at((18, 3)), Some(Control::Fill));
        assert_eq!(Scene::control_at((26, 3)), Some(Control::Play));
        assert_eq!(Scene::control_at((30, 3)), None);
        assert_eq!(Scene::control_at((2, 8)), None);
    }

    #[test]
    fn disabled_controls_retain_their_state() {
        let mut scene = Scene::new();
        scene.kind = Kind::Circle;
        scene.activate_control(Control::AntiAlias);
        scene.activate_control(Control::Fill);
        assert!(scene.anti_alias && scene.fill);

        scene.activate_control(Control::Shape);
        assert_eq!(scene.kind, Kind::Ellipse);
        assert!(scene.anti_alias && scene.fill);

        scene.activate_control(Control::Shape);
        assert_eq!(scene.kind, Kind::QuadBezier);
        assert!(scene.fill, "fill state was not retained");
        scene.activate_control(Control::Fill);
        assert!(scene.fill, "disabled fill toggle changed state");

        scene.activate_control(Control::Shape);
        assert_eq!(scene.kind, Kind::ThickLine);
        assert!(scene.anti_alias, "AA state was not retained");
        assert!(scene.fill, "fill state was not retained");
        scene.activate_control(Control::AntiAlias);
        assert!(!scene.anti_alias, "AA toggle should work on ThickLine");
        scene.activate_control(Control::Fill);
        assert!(!scene.fill, "fill toggle should work on ThickLine");

        scene.activate_control(Control::Shape);
        assert_eq!(scene.kind, Kind::RoundRect);
        assert!(!scene.anti_alias, "AA state was not retained");
        assert!(!scene.fill, "fill state was not retained");
        scene.activate_control(Control::AntiAlias);
        assert!(scene.anti_alias, "AA toggle should work on RoundRect");
        scene.activate_control(Control::Fill);
        assert!(scene.fill, "fill toggle should work on RoundRect");
    }

    #[test]
    fn disabled_icons_are_grey() {
        let mut scene = Scene::new();
        scene.clear();
        scene.stamp_digit();

        let pixel = |scene: &Scene, x: usize, y: usize| {
            let i = (y * super::WIDTH as usize + x) * 4;
            [
                scene.buf[i],
                scene.buf[i + 1],
                scene.buf[i + 2],
                scene.buf[i + 3],
            ]
        };
        assert_eq!(pixel(&scene, 10, 1), [255, 255, 255, 255]);
        assert_eq!(pixel(&scene, 20, 3), [0x55, 0x55, 0x55, 255]);
        assert_eq!(pixel(&scene, 14, 3), BG);
    }

    #[test]
    fn play_pause_icons_differ() {
        let mut scene = Scene::new();
        scene.clear();
        scene.stamp_play(true);
        let pixel = |scene: &Scene, x: usize, y: usize| {
            let i = (y * super::WIDTH as usize + x) * 4;
            [
                scene.buf[i],
                scene.buf[i + 1],
                scene.buf[i + 2],
                scene.buf[i + 3],
            ]
        };
        assert_eq!(pixel(&scene, 25, 3), [255, 255, 255, 255]);
        assert_eq!(pixel(&scene, 26, 3), BG);
        assert_eq!(pixel(&scene, 27, 3), [255, 255, 255, 255]);

        scene.stamp_play(false);
        assert_eq!(pixel(&scene, 25, 3), [255, 255, 255, 255]);
        assert_eq!(pixel(&scene, 26, 3), [255, 255, 255, 255]);
        assert_eq!(pixel(&scene, 27, 3), [255, 255, 255, 255]);
    }

    #[test]
    fn round_rect_control_sets_radius_from_top_left() {
        let mut scene = Scene::new();
        scene.kind = Kind::RoundRect;
        scene.start = (10, 10);
        scene.end = (40, 34);
        scene.reset_control();
        assert_eq!(scene.round_rect_radius(), 6);
        assert_eq!(scene.handle_point(), (16, 16));

        scene.control = (14, 19);
        assert_eq!(scene.round_rect_radius(), 4);
        assert_eq!(scene.handle_point(), (14, 14));
    }

    #[test]
    fn thick_line_control_sets_width_from_offset() {
        let mut scene = Scene::new();
        scene.kind = Kind::ThickLine;
        scene.start = (10, 20);
        scene.end = (40, 20);
        scene.reset_control();
        assert_eq!(scene.thick_line_width(), 3.0);
        assert_eq!(scene.handle_point(), (25, 23));

        scene.control = (18, 26);
        assert_eq!(scene.thick_line_width(), 6.0);
        assert_eq!(scene.handle_point(), (25, 26));

        scene.control = (25, 14);
        assert_eq!(scene.thick_line_width(), 6.0);
        assert_eq!(scene.handle_point(), (25, 14));
    }
}
