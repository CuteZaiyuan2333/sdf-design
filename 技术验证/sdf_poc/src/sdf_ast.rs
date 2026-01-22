use rhai::{Engine, CustomType, TypeBuilder};

// --- The Core Data Structure (AST) ---

#[derive(Clone, Debug)]
pub enum SdfOp {
    // Primitives
    Sphere { radius: f32 },
    Box { size: [f32; 3] }, // Half-extents usually
    
    // Boolean Operations
    Union { a: Box<SdfNode>, b: Box<SdfNode>, smooth: f32 },
    Subtract { a: Box<SdfNode>, b: Box<SdfNode>, smooth: f32 },
    Intersect { a: Box<SdfNode>, b: Box<SdfNode>, smooth: f32 },
    
    // Transforms
    Translate { target: Box<SdfNode>, offset: [f32; 3] },
    Rotate { target: Box<SdfNode>, axis: [f32; 3], angle_deg: f32 },
}

#[derive(Clone, Debug)]
pub struct SdfNode {
    pub op: SdfOp,
}

impl SdfNode {
    // --- Constructors for Rhai ---
    
    pub fn new_sphere(radius: f32) -> Self {
        Self { op: SdfOp::Sphere { radius } }
    }

    pub fn new_box(x: f32, y: f32, z: f32) -> Self {
        Self { op: SdfOp::Box { size: [x, y, z] } }
    }

    // --- Methods for Fluent Interface ---

    pub fn union(&mut self, other: SdfNode) -> SdfNode {
        Self {
            op: SdfOp::Union { 
                a: Box::new(self.clone()), 
                b: Box::new(other), 
                smooth: 0.0 
            }
        }
    }
    
    pub fn smooth_union(&mut self, other: SdfNode, k: f32) -> SdfNode {
        Self {
            op: SdfOp::Union { 
                a: Box::new(self.clone()), 
                b: Box::new(other), 
                smooth: k
            }
        }
    }

    pub fn subtract(&mut self, other: SdfNode) -> SdfNode {
        Self {
            op: SdfOp::Subtract { 
                a: Box::new(self.clone()), 
                b: Box::new(other), 
                smooth: 0.0 
            }
        }
    }
    
    pub fn smooth_subtract(&mut self, other: SdfNode, k: f32) -> SdfNode {
        Self {
            op: SdfOp::Subtract { 
                a: Box::new(self.clone()), 
                b: Box::new(other), 
                smooth: k
            }
        }
    }

    pub fn intersect(&mut self, other: SdfNode) -> SdfNode {
        Self {
            op: SdfOp::Intersect { 
                a: Box::new(self.clone()), 
                b: Box::new(other), 
                smooth: 0.0 
            }
        }
    }

    pub fn translate(&mut self, x: f32, y: f32, z: f32) -> SdfNode {
        Self {
            op: SdfOp::Translate { 
                target: Box::new(self.clone()), 
                offset: [x, y, z] 
            }
        }
    }

    pub fn rotate_x(&mut self, deg: f32) -> SdfNode {
        Self {
            op: SdfOp::Rotate { 
                target: Box::new(self.clone()), 
                axis: [1.0, 0.0, 0.0],
                angle_deg: deg 
            }
        }
    }
    
    pub fn rotate_y(&mut self, deg: f32) -> SdfNode {
        Self {
            op: SdfOp::Rotate { 
                target: Box::new(self.clone()), 
                axis: [0.0, 1.0, 0.0],
                angle_deg: deg 
            }
        }
    }
    
    pub fn rotate_z(&mut self, deg: f32) -> SdfNode {
        Self {
            op: SdfOp::Rotate { 
                target: Box::new(self.clone()), 
                axis: [0.0, 0.0, 1.0],
                angle_deg: deg 
            }
        }
    }
}

// Rhai Registration Helper
impl CustomType for SdfNode {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("SdfNode")
            .with_fn("union", SdfNode::union)
            .with_fn("add", SdfNode::union) // Alias
            .with_fn("smooth_union", SdfNode::smooth_union)
            .with_fn("subtract", SdfNode::subtract)
            .with_fn("sub", SdfNode::subtract) // Alias
            .with_fn("smooth_subtract", SdfNode::smooth_subtract)
            .with_fn("intersect", SdfNode::intersect)
            .with_fn("translate", SdfNode::translate)
            .with_fn("move", SdfNode::translate) // Alias
            .with_fn("rotate_x", SdfNode::rotate_x)
            .with_fn("rotate_y", SdfNode::rotate_y)
            .with_fn("rotate_z", SdfNode::rotate_z);
    }
}

pub fn register_rhai_types(engine: &mut Engine) {
    engine.build_type::<SdfNode>();
    
    // Register constructors as global functions
    engine.register_fn("sphere", SdfNode::new_sphere);
    engine.register_fn("box", SdfNode::new_box);
    engine.register_fn("cube", SdfNode::new_box); // Alias
}
