# nano9_raster
<img align="right" width="256" height="192" alt="demo" src="https://github.com/user-attachments/assets/91c54fe3-9560-49fa-b77e-536f5d6dbffd" />

Iterator-based
[Bresenham](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm)'s line,
circle, and more drawing algorithms.

Bresenham's line drawing algorithm is a fast algorithm to draw a line between
two points, without any overdraw. This crate implements the fast integer
variant, using an iterator-based approach for flexibility. Most, if not all,
overhead should evaporate when inlined by the compiler. It calculates
coordinates without knowing anything about drawing methods or surfaces.



## Example

```rust
for (x, y) in nano9_raster::Line::new((0, 1), (6, 4)) {
    println!("({}, {})", x, y);
}
```

```text
(0, 1)
(1, 1)
(2, 2)
(3, 2)
(4, 3)
(5, 3)
```

## Shapes

Bresenham published algorithms for lines and circles. And a number of other
shapes were generalized from his work (see references below), which are
available as optional Cargo features.

| Demo | Shape            | AA option | fillable | Author              | Feature                         |
|------|------------------|-----------|----------|---------------------|---------------------------------|
| 0    | Line             | X         |          | Bresenham, Pitteway | `line`, `aa`                    |
| 1    | Circle           | X         | X        | Bresenham, Fu       | `circle`, `aa`, `fill`          |
| 2    | Ellipse          | X         | X        | Pitteway, Vadillo   | `ellipse`, `aa`, `fill`         |
| 3    | Quadratic Bézier | X         |          | Zingl               | `bezier`, `aa`                  |
| 4    | Thick line       | X         | X        | Murphy, Zingl, Celis | `thick-line`, `aa`, `murphy`, `celis` |
| 5    | Rounded rect     | X         | X        |                     | `round-rect`, `aa`, `fill`      |
| -    | 3D Line          |           |          | Kaufman             | `line3d`                        |


## Demo

The WASM demo shows a 64×48 canvas that autoplays every applicable combination
of the six numbered shapes above. Three controls run along the top: click the
number to advance the shape, the small circle to toggle anti-aliasing, or the
outline/filled circle to toggle filling. An inapplicable control is grey and
its setting is retained for the next applicable shape. `Line3d` and the
center-and-radii `Ellipse` are not shown in the demo.

```sh
cd demo
trunk serve; # Open the URL trunk prints.
```

Left-drag draws the current shape between two points. On a Bézier, thick line,
or rounded rect, a second drag moves the orange control point (curve handle,
stroke width, or corner radius); later drags near that handle move it again.
Right-click stops
autoplay and advances to the next shape. After a few seconds of inactivity,
autoplay resumes.

## Boundaries

The boundaries half-open or inclusive were chosen for performance,
composability, and ease to convert one to the other. Converting from half-open
to inclusive is furnished by the `Inclusive` trait. An inclusive interval may be
created half-open by dropping its last point; however, this library does not
provide that convenience because it incurs a small performance penalty per step,
and one of the authors maintains that performance degradation should not be made
convenient.

## Fill

The `fill` Cargo feature adds the `Fill` trait on `Circle`, `CircleAa`,
`Ellipse`, `EllipseAa`, `RoundRect`, and `RoundRectAa`. Ellipses can be constructed from a center and radii
with `new`, or from opposite bounding-rectangle corners with `from_rect`. The
trait is generic on its iterator item and defaults to `Span`: one solid
inclusive `[x0, x1]` chord per distinct row. The AA shapes implement
`Fill<Plot>` and mix those spans with `Point`s carrying anti-aliased rim
coverage. The circle uses Vadillo's integer algorithm; the ellipses extend its
squared implicit-function band with the ellipse's local gradient. Even-sized
rectangle bounds produce exactly the corresponding center-and-radii
`EllipseAa` result.

## Inclusive

The `inclusive` Cargo feature adds the `Inclusive` trait on `Line` and `Line3d`,
which returns an iterator that includes the end point making the interval
inclusive `[start, end]` instead of half-open. This approach
follows [indubitablement2's
work](https://github.com/indubitablement2/bresenham-rs), which is careful not
to incur any runtime penalty.
 
### Inclusive Example

This code will produce the same points as the first example plus `(6, 4)` at the
end.

```rust,ignore
use nano9_raster::Inclusive;

for (x, y) in nano9_raster::Line::new((0, 1), (6, 4)).inclusive() {
    println!("({}, {})", x, y);
}
```

Note: `line.inclusive()` is semantically equivalent to doing `Line::new(start,
end).chain(iter::once(end))` but it does not have any iterator chaining
overhead.

## Bresenham Line Variant Notes

By default lines are drawn on a half-open interval `[start, end)`: the `start`
point is included, but the `end` point is not. This allows one to chain multiple
lines together without any overdraw. However, one can opt-in to the "inclusive"
Cargo feature described above.

This particular implementation of Bresenham breaks ties in quadrants such that
an inclusive line drawn from `(A, B)` and from `(B, A)` will cover the same
points. Thus lines are symmetrical.

## Why No Generics?

Other Bresenham implementations use a generic type, and there are [forks of this
project](https://github.com/nsmryan/bresenham-rs/commit/4ccd83759d92bd243f48b52be6826df252a88578)
that switch to `i32` instead of the fixed `isize`. As an experiment, the `Line`
was changed to be generic and benchmarked against different signed types.

| Type    | half-open (horizontal) | vs `isize`   |
|---------|------------------------|--------------|
| `i8`    | 1.07 µs                | 1.40× slower |
| `i16`   | 1.07 µs                | 1.40× slower |
| `i32`   | 780 ns                 | 1.02× slower |
| `i64`   | 766 ns                 | same         |
| `isize` | 766 ns                 | —            |
| `i128`  | 1.32 µs                | 1.73× slower |

The types `i8` and `i16` losing to `i32` and `i64` is probably due to LLVM's
extra extend and truncate. Unfortunately, smaller is not faster, but bigger is
definitely slower as with `i128`. Finally `isize` is the machine-size word, so
one can expect it will be the fastest. Based on this experiment, adding generics
was rejected.

## References

- J. E. Bresenham, ["Algorithm for computer control of a
  digital plotter"](https://doi.org/10.1147/sj.41.0025), *IBM Systems Journal*,
  4(1):25–30, 1965.
  <a href="https://cdn.jsdelivr.net/gh/shanecelis/nano9_raster@main/doc/papers/bresenham-1965-line.pdf" target="_blank" rel="noopener noreferrer">PDF</a> `Line`
- J. Bresenham, ["A linear algorithm for incremental digital display of circular
  arcs"](https://doi.org/10.1145/359423.359432), *Communications of the ACM*,
  20(2):100–106, 1977.
  <a href="https://cdn.jsdelivr.net/gh/shanecelis/nano9_raster@main/doc/papers/bresenham-1977-circle.pdf" target="_blank" rel="noopener noreferrer">PDF</a> `Circle`
- M. L. V. Pitteway, ["Algorithm for drawing ellipses or hyperbolae with a
  digital plotter"](https://doi.org/10.1093/comjnl/10.3.282), *The Computer
  Journal*, 10(3):282–289, 1967.
  <a href="https://cdn.jsdelivr.net/gh/shanecelis/nano9_raster@main/doc/papers/pitteway-1967-ellipse.pdf" target="_blank" rel="noopener noreferrer">PDF</a> `Ellipse`
- M. L. V. Pitteway and D. J. Watkinson, ["Bresenham's algorithm with Grey
  scale"](https://doi.org/10.1145/359024.359027), *Communications of the ACM*,
  23(11):625–626, 1980.
  <a href="https://cdn.jsdelivr.net/gh/shanecelis/nano9_raster@main/doc/papers/pitteway-watkinson-1980-grey-scale.pdf" target="_blank" rel="noopener noreferrer">PDF</a> `LineAa`
- A. S. Murphy, ["Line Thickening by Modification to Bresenham's
  Algorithm"](http://homepages.enterprise.net/murphy/thickline/index.html),
  *IBM Technical Disclosure Bulletin*, 20(12):5358–5366, 1978.
  <a href="https://cdn.jsdelivr.net/gh/shanecelis/nano9_raster@main/doc/papers/murphy-1978-thickline.pdf" target="_blank" rel="noopener noreferrer">PDF</a> `ThickLine` `ThickLineAa`; `murphy::ThickLineFill`
- A. E. Kaufman and E. Shimony, ["3D scan-conversion algorithms for voxel-based
  graphics"](https://doi.org/10.1145/319120.319126), *Proceedings of the 1986
  Workshop on Interactive 3D Graphics*, 45–75, 1986.
  <a href="https://cdn.jsdelivr.net/gh/shanecelis/nano9_raster@main/doc/papers/kaufman-shimony-1986-3d-scan-conversion.pdf" target="_blank" rel="noopener noreferrer">PDF</a> `Line3d`
- A. Zingl, ["A Rasterizing Algorithm for Drawing
  Curves"](https://zingl.github.io/Bresenham.pdf), Technikum Wien, 2012.
  <a href="https://cdn.jsdelivr.net/gh/shanecelis/nano9_raster@main/doc/papers/zingl-2012-rasterizing-curves.pdf" target="_blank" rel="noopener noreferrer">PDF</a> [Site](http://members.chello.at/easyfilter/bresenham.html) [Code](http://members.chello.at/easyfilter/bresenham.c) `QuadBezier` `QuadBezierAa` `ThickLineFill` `ThickLineFillAa`
- B. Fu and L. Niu, ["Integral Algorithm for Generating Anti-Aliasing Circle
  Based on Bresenham Algorithm"](https://doi.org/10.4028/www.scientific.net/AMR.490-495.1202),
  *Advanced Materials Research*, 490–495:1202–1206, 2012.
  <a href="https://cdn.jsdelivr.net/gh/shanecelis/nano9_raster@main/doc/papers/fu-niu-2012-antialiasing-circle.pdf" target="_blank" rel="noopener noreferrer">PDF</a> `CircleAa`
- J. R. Vadillo, ["A novel technique to draw antialiased circles without
  floating point math nor square root"](https://github.com/Versa-Design/Antialiased_Circle),
  Versa Design S.L., 2023.
  <a href="https://cdn.jsdelivr.net/gh/shanecelis/nano9_raster@main/doc/papers/vadillo-2023-antialiased-circle.pdf" target="_blank" rel="noopener noreferrer">PDF</a> `CircleAa` + `Fill`, inspiration for `EllipseAa` + `Fill`

## License

This crate is licensed under the MIT License.

## Acknowledgments

Many thanks to [Marc Brinkmann](https://github.com/mbr) for creating the
original [`bresenham`](https://github.com/mbr/bresenham-rs) crate, from which
`nano9_raster` was
[inspired](https://mastodon.gamedev.place/@shanecelis/117135093640227966) and
forked after a [PR](https://github.com/mbr/bresenham-rs/pull/6) was submitted
and withdrawn.
