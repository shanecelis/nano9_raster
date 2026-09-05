#![doc = include_str!("../README.md")]
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(test)]
extern crate std;

#[cfg(feature = "bezier")]
mod bezier;
#[cfg(feature = "circle")]
mod circle;
#[cfg(all(feature = "aa", any(feature = "circle", feature = "ellipse")))]
mod circle_aa;
#[cfg(feature = "ellipse")]
mod ellipse;
#[cfg(all(feature = "aa", feature = "ellipse"))]
mod ellipse_aa;
#[cfg(feature = "fill")]
mod fill;
#[cfg(feature = "inclusive")]
mod inclusive;
#[cfg(feature = "line")]
mod line;
#[cfg(feature = "line3d")]
mod line3d;
#[cfg(all(feature = "aa", feature = "line"))]
mod line_aa;
#[cfg(all(feature = "aa", feature = "bezier"))]
mod quad_bezier_aa;
#[cfg(feature = "round-rect")]
mod round_rect;
#[cfg(feature = "wide-line")]
mod wide_line;
#[cfg(all(feature = "aa", feature = "wide-line"))]
mod wide_line_aa;

#[cfg(feature = "bezier")]
#[cfg_attr(docsrs, doc(cfg(feature = "bezier")))]
pub use bezier::QuadBezier;
#[cfg(feature = "circle")]
#[cfg_attr(docsrs, doc(cfg(feature = "circle")))]
pub use circle::Circle;
#[cfg(all(feature = "aa", feature = "circle"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "aa", feature = "circle"))))]
pub use circle_aa::CircleAa;
#[cfg(feature = "ellipse")]
#[cfg_attr(docsrs, doc(cfg(feature = "ellipse")))]
pub use ellipse::Ellipse;
#[cfg(all(feature = "aa", feature = "ellipse"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "aa", feature = "ellipse"))))]
pub use ellipse_aa::EllipseAa;
#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(feature = "fill")))]
pub use fill::{Fill, Plot, Span};
#[cfg(feature = "inclusive")]
#[cfg_attr(docsrs, doc(cfg(feature = "inclusive")))]
pub use inclusive::Inclusive;
#[cfg(feature = "line")]
#[cfg_attr(docsrs, doc(cfg(feature = "line")))]
pub use line::{Bresenham, Line};
#[cfg(feature = "line3d")]
#[cfg_attr(docsrs, doc(cfg(feature = "line3d")))]
pub use line3d::Line3d;
#[cfg(all(feature = "aa", feature = "line"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "aa", feature = "line"))))]
pub use line_aa::LineAa;
#[cfg(all(feature = "aa", feature = "bezier"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "aa", feature = "bezier"))))]
pub use quad_bezier_aa::QuadBezierAa;
#[cfg(feature = "round-rect")]
#[cfg_attr(docsrs, doc(cfg(feature = "round-rect")))]
pub use round_rect::RoundRect;
#[cfg(feature = "wide-line")]
#[cfg_attr(docsrs, doc(cfg(feature = "wide-line")))]
pub use wide_line::WideLine;
#[cfg(all(feature = "aa", feature = "wide-line"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "aa", feature = "wide-line"))))]
pub use wide_line_aa::WideLineAa;

/// Convenient typedef for two machine-sized integers
pub type Point = (isize, isize);

/// A point plus its anti-alias coverage
///
/// `255` is fully on; `0` is fully off.
pub type PointAa = (Point, u8);

/// Convenient typedef for three machine-sized integers
#[cfg(feature = "line3d")]
#[cfg_attr(docsrs, doc(cfg(feature = "line3d")))]
pub type Point3d = (isize, isize, isize);
