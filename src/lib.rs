use lyon_tessellation::{StrokeVertexConstructor, FillVertexConstructor, StrokeTessellator, FillTessellator, BuffersBuilder, StrokeOptions, StrokeVertex, FillOptions, FillVertex};
use lyon_tessellation::math::{Point, Box2D, Vector, Angle};
use lyon_path::{builder::{Build, PathBuilder}, Winding};

pub use lyon_tessellation::VertexBuffers;

pub trait Attributes: bytemuck::Pod {
    fn from_f32_bytes(bytes: &[f32]) -> Self where Self: Sized {
        *bytemuck::from_bytes(&bytes.iter().copied().map(|f| f as u8).collect::<Vec<_>>())
    }

    fn to_f32_bytes(&self) -> Vec<f32> {
        bytemuck::bytes_of(self).iter().copied().map(|u| u as f32).collect::<Vec<_>>()
    }
}
impl<T: bytemuck::Pod> Attributes for T {}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy)]
pub enum DrawCommand<A: Attributes> {
    QuadraticBezierTo(A, f32, f32, f32, f32), //x, y, ctrlx, ctrly
    CubicBezierTo(A, f32, f32, f32, f32, f32, f32), //x, y, ctrlx, ctrly, ctrlx2, ctrly2
    LineTo(A, f32, f32), //x, y
}

pub enum Shape<A: Attributes> {
    Draw(A, f32, f32, Vec<DrawCommand<A>>),
    RoundedRectangle(A, f32, f32, f32, f32, f32, f32),
    Rectangle(A, f32, f32, f32, f32),
    Ellipse(A, f32, f32, f32, f32)
}

impl<A: Attributes> Shape<A> {
    fn attrs_len(&self) -> usize {
        match self {
            Shape::RoundedRectangle(attrs, ..) => attrs,
            Shape::Rectangle(attrs, ..) => attrs,
            Shape::Ellipse(attrs, ..) => attrs,
            Shape::Draw(attrs, ..) => attrs,
        }.to_f32_bytes().len()
    }
}

pub struct ShapeBuilder<A: Attributes> {
    shape: Shape<A>,
    stroke_width: Option<f32>,
    tolerance: f32,
}

impl<A: Attributes> ShapeBuilder<A> {
    pub fn new(shape: Shape<A>, stroke_width: Option<f32>, tolerance: f32) -> Self {
        ShapeBuilder{
            shape, stroke_width, tolerance
        }
    }
    pub fn build<V: Vertex>(self, buffer: &mut VertexBuffers<V, u16>) {
        match self.stroke_width {
            Some(sw) => {
                let mut tessellator = StrokeTessellator::new();
                let o = StrokeOptions::default().with_tolerance(self.tolerance).with_line_width(sw);
                let mut buffers = BuffersBuilder::new(buffer, Constructor(Box::new(V::construct)));
                let mut builder = tessellator.builder_with_attributes(
                    self.shape.attrs_len(), &o, &mut buffers
                );
                self.build_shape(&mut builder);
                builder.build().unwrap()
            },
            None => {
                let mut tessellator = FillTessellator::new();
                let options = FillOptions::default().with_tolerance(self.tolerance);
                let mut buffers = BuffersBuilder::new(buffer, Constructor(Box::new(V::construct)));
                let mut builder = tessellator.builder_with_attributes(
                    self.shape.attrs_len(), &options, &mut buffers
                );
                self.build_shape(&mut builder);
                builder.build().unwrap()
            }
        }
    }

    fn build_shape(self, builder: &mut dyn PathBuilder) {
        match self.shape {
            Shape::Draw(a, x, y, commands) => {
                builder.begin(Point::new(x, y), &a.to_f32_bytes());
                commands.into_iter().for_each(|command| match command {
                        DrawCommand::QuadraticBezierTo(a, x, y, ctrlx, ctrly) => {
                            let tp = Point::new(x, y);
                            let cp = Point::new(ctrlx, ctrly);
                            builder.quadratic_bezier_to(cp, tp, &a.to_f32_bytes());
                        },
                        DrawCommand::CubicBezierTo(a, x, y, ctrlx, ctrly, ctrlx2, ctrly2) => {
                            let tp = Point::new(x, y);
                            let cp = Point::new(ctrlx, ctrly);
                            let cp2 = Point::new(ctrlx2, ctrly2);
                            builder.cubic_bezier_to(cp, cp2, tp, &a.to_f32_bytes());
                        },
                        DrawCommand::LineTo(a, x, y) => {
                            let tp = Point::new(x, y);
                            builder.line_to(tp, &a.to_f32_bytes());
                        }
                });
                builder.close();
            },
            Shape::RoundedRectangle(_a, _x, _y, _x2, _y2, _xr, _yr) => {
                todo!();
            },
            Shape::Rectangle(a, x, y, x2, y2) => {
                builder.add_rectangle(
                    &Box2D::new(
                        Point::new(x, y),
                        Point::new(x2, y2),
                    ),
                    Winding::Positive,
                    &a.to_f32_bytes()
                );
            },
            Shape::Ellipse(a, x, y, xr, yr) => {
                builder.add_ellipse(
                    Point::new(x, y),
                    Vector::new(xr, yr),
                    Angle::radians(0.0),
                    Winding::Positive,
                    &a.to_f32_bytes()
                );
            }
        }
    }
}

pub trait Vertex: bytemuck::Pod {
    type Attributes: Attributes;
    fn construct(position: [f32; 2], attrs: Self::Attributes) -> Self where Self: Sized;
}

struct Constructor<V: Vertex>(Box<dyn Fn([f32; 2], V::Attributes) -> V>);

impl<V: Vertex> StrokeVertexConstructor<V> for Constructor<V> {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> V {
        (self.0)(vertex.position().to_array(), V::Attributes::from_f32_bytes(vertex.interpolated_attributes()))
    }
}

impl<V: Vertex> FillVertexConstructor<V> for Constructor<V> {
    fn new_vertex(&mut self, mut vertex: FillVertex) -> V {
        (self.0)(vertex.position().to_array(), V::Attributes::from_f32_bytes(vertex.interpolated_attributes()))
    }
}
