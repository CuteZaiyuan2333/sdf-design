use crate::sdf_ast::{SdfNode, SdfOp};

pub struct WgslGenerator {
    code: String,
    // Variable counter to ensure unique names if we need temporary vars
    // For now we will generate a single nested expression which is cleaner for simple trees.
}

impl WgslGenerator {
    pub fn new() -> Self {
        Self { code: String::new() }
    }

    pub fn generate(&mut self, root: &SdfNode) -> String {
        // We generate the body of the 'map' function.
        // Input variable is 'p_in'.
        // Output is a float.
        
        let expression = self.emit_expression(root, "p_in");
        
        format!(
            "
            fn map(p_in: vec3<f32>) -> f32 {{
                return {};
            }}
            ",
            expression
        )
    }

    fn emit_expression(&self, node: &SdfNode, p_var: &str) -> String {
        match &node.op {
            SdfOp::Sphere { radius } => {
                format!("sd_sphere({p_var}, {radius:.4})")
            }
            SdfOp::Box { size } => {
                format!("sd_box({p_var}, vec3<f32>({:.4}, {:.4}, {:.4}))", size[0], size[1], size[2])
            }
            SdfOp::Union { a, b, smooth } => {
                let d1 = self.emit_expression(a, p_var);
                let d2 = self.emit_expression(b, p_var);
                if *smooth > 0.0 {
                    format!("op_smooth_union({d1}, {d2}, {smooth:.4})")
                } else {
                    format!("min({d1}, {d2})")
                }
            }
            SdfOp::Subtract { a, b, smooth } => {
                // Subtract b from a: max(a, -b)
                let d1 = self.emit_expression(a, p_var);
                let d2 = self.emit_expression(b, p_var);
                if *smooth > 0.0 {
                    // Smooth subtraction is a bit trickier, standard implementation:
                    // mix(a, -b, h) ... but usually we defined op_smooth_union.
                    // Let's assume op_smooth_subtraction is needed or we just invert d2.
                    // For now, let's stick to standard crisp subtraction or implement a smooth sub helper in shader.
                    // Let's use crisp for simplicity unless I add a helper.
                    // Update: I'll use standard min(d1, -d2) for crisp.
                    format!("max({d1}, -{d2})")
                } else {
                    format!("max({d1}, -{d2})")
                }
            }
            SdfOp::Intersect { a, b, smooth: _ } => {
                let d1 = self.emit_expression(a, p_var);
                let d2 = self.emit_expression(b, p_var);
                format!("max({d1}, {d2})")
            }
            SdfOp::Translate { target, offset } => {
                // p' = p - offset
                let new_p = format!("({p_var} - vec3<f32>({:.4}, {:.4}, {:.4}))", offset[0], offset[1], offset[2]);
                self.emit_expression(target, &new_p)
            }
            SdfOp::Rotate { target, axis, angle_deg } => {
                // p' = rotate(p, -angle)  <-- Inverse transform for SDF
                // We need a rotation function in shader. I have rotate_y.
                // For arbitrary axis, we'd need a general rotation matrix or quaternion.
                // For this POC, let's strictly support X, Y, Z by generating specific calls.
                
                let rad = angle_deg.to_radians();
                // Note: We use negative angle for inverse transform
                let neg_rad = -rad;
                
                if axis[1] > 0.9 { // Y axis
                     let new_p = format!("rotate_y({p_var}, {:.4})", neg_rad);
                     return self.emit_expression(target, &new_p);
                } else if axis[0] > 0.9 { // X axis
                     // We need rotate_x in shader. I'll add it to the boilerplate.
                     let new_p = format!("rotate_x({p_var}, {:.4})", neg_rad);
                     return self.emit_expression(target, &new_p);
                } else { // Z axis
                     let new_p = format!("rotate_z({p_var}, {:.4})", neg_rad);
                     return self.emit_expression(target, &new_p);
                }
            }
        }
    }
}
