use rhai::{Engine, CustomType, TypeBuilder};

#[derive(Clone, Debug)]
pub enum SdfOp {
    Sphere { radius: f32 },
    Box { size: [f32; 3] },
    Cylinder { radius: f32, height: f32 },
    Torus { major_radius: f32, minor_radius: f32 },
    
    Union { a: Box<SdfNode>, b: Box<SdfNode>, smooth: f32 },
    Subtract { a: Box<SdfNode>, b: Box<SdfNode>, smooth: f32 },
    #[allow(dead_code)]
    Intersect { a: Box<SdfNode>, b: Box<SdfNode>, smooth: f32 },
    
    Translate { target: Box<SdfNode>, offset: [f32; 3] },
    Rotate { target: Box<SdfNode>, axis: [f32; 3], angle_deg: f32 },
    Mirror { target: Box<SdfNode>, axis: [f32; 3] },
    
    Color { target: Box<SdfNode>, color: [f32; 3] },
}

#[derive(Clone, Debug)]
pub struct SdfNode {
    pub op: SdfOp,
}

impl SdfNode {
    pub fn new_sphere(radius: f32) -> Self { Self { op: SdfOp::Sphere { radius } } }
    pub fn new_box(x: f32, y: f32, z: f32) -> Self { Self { op: SdfOp::Box { size: [x, y, z] } } }
    pub fn new_cylinder(r: f32, h: f32) -> Self { Self { op: SdfOp::Cylinder { radius: r, height: h } } }
    pub fn new_torus(major: f32, minor: f32) -> Self { Self { op: SdfOp::Torus { major_radius: major, minor_radius: minor } } }

    pub fn union(&mut self, other: SdfNode) -> SdfNode { Self { op: SdfOp::Union { a: Box::new(self.clone()), b: Box::new(other), smooth: 0.0 } } }
    pub fn smooth_union(&mut self, other: SdfNode, k: f32) -> SdfNode { Self { op: SdfOp::Union { a: Box::new(self.clone()), b: Box::new(other), smooth: k } } }
    pub fn subtract(&mut self, other: SdfNode) -> SdfNode { Self { op: SdfOp::Subtract { a: Box::new(self.clone()), b: Box::new(other), smooth: 0.0 } } }
    #[allow(dead_code)]
    pub fn smooth_subtract(&mut self, other: SdfNode, k: f32) -> SdfNode { Self { op: SdfOp::Subtract { a: Box::new(self.clone()), b: Box::new(other), smooth: k } } }
    
    pub fn translate(&mut self, x: f32, y: f32, z: f32) -> SdfNode { Self { op: SdfOp::Translate { target: Box::new(self.clone()), offset: [x, y, z] } } }
    pub fn rotate_x(&mut self, deg: f32) -> SdfNode { Self { op: SdfOp::Rotate { target: Box::new(self.clone()), axis: [1.0, 0.0, 0.0], angle_deg: deg } } }
    pub fn rotate_y(&mut self, deg: f32) -> SdfNode { Self { op: SdfOp::Rotate { target: Box::new(self.clone()), axis: [0.0, 1.0, 0.0], angle_deg: deg } } }
    pub fn rotate_z(&mut self, deg: f32) -> SdfNode { Self { op: SdfOp::Rotate { target: Box::new(self.clone()), axis: [0.0, 0.0, 1.0], angle_deg: deg } } }
    
    pub fn mirror_x(&mut self) -> SdfNode { Self { op: SdfOp::Mirror { target: Box::new(self.clone()), axis: [1.0, 0.0, 0.0] } } }
    pub fn mirror_y(&mut self) -> SdfNode { Self { op: SdfOp::Mirror { target: Box::new(self.clone()), axis: [0.0, 1.0, 0.0] } } }
    pub fn mirror_z(&mut self) -> SdfNode { Self { op: SdfOp::Mirror { target: Box::new(self.clone()), axis: [0.0, 0.0, 1.0] } } }

    pub fn color(&mut self, r: f32, g: f32, b: f32) -> SdfNode { 
        Self { op: SdfOp::Color { target: Box::new(self.clone()), color: [r, g, b] } } 
    }
}

impl CustomType for SdfNode {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("SdfNode")
            .with_fn("union", SdfNode::union).with_fn("add", SdfNode::union)
            .with_fn("smooth_union", SdfNode::smooth_union)
            .with_fn("subtract", SdfNode::subtract).with_fn("sub", SdfNode::subtract)
            .with_fn("translate", SdfNode::translate).with_fn("move", SdfNode::translate)
            .with_fn("rotate_x", SdfNode::rotate_x)
            .with_fn("rotate_y", SdfNode::rotate_y)
            .with_fn("rotate_z", SdfNode::rotate_z)
            .with_fn("mirror_x", SdfNode::mirror_x)
            .with_fn("mirror_y", SdfNode::mirror_y)
            .with_fn("mirror_z", SdfNode::mirror_z)
            .with_fn("color", SdfNode::color);
    }
}

pub fn register_rhai_types(engine: &mut Engine) {
    engine.build_type::<SdfNode>();
    engine.register_fn("sphere", SdfNode::new_sphere);
    engine.register_fn("box", SdfNode::new_box);
    engine.register_fn("cylinder", SdfNode::new_cylinder);
    engine.register_fn("torus", SdfNode::new_torus);
}