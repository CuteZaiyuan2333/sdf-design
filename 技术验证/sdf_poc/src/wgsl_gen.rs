use crate::sdf_ast::{SdfNode, SdfOp};

pub struct WgslGenerator {
    _unused: bool,
}

impl WgslGenerator {
    pub fn new() -> Self {
        Self { _unused: true }
    }

    pub fn generate(&mut self, root: &SdfNode) -> String {
        let expression = self.emit_expression(root, "p_in");
        format!(
            "fn map(p_in: vec3<f32>) -> f32 {{
                return {};
            }}",
            expression
        )
    }

    fn emit_expression(&self, node: &SdfNode, p_var: &str) -> String {
        match &node.op {
            SdfOp::Sphere { radius } => format!("sd_sphere({p_var}, {radius:.4})"),
            SdfOp::Box { size } => format!("sd_box({p_var}, vec3<f32>({:.4}, {:.4}, {:.4}))", size[0], size[1], size[2]),
            SdfOp::Cylinder { radius, height } => format!("sd_cylinder({p_var}, {radius:.4}, {height:.4})"),
            SdfOp::Torus { major_radius, minor_radius } => format!("sd_torus({p_var}, vec2<f32>({major_radius:.4}, {minor_radius:.4}))"),
            
            SdfOp::Union { a, b, smooth } => {
                let d1 = self.emit_expression(a, p_var);
                let d2 = self.emit_expression(b, p_var);
                if *smooth > 0.0 { format!("op_smooth_union({d1}, {d2}, {smooth:.4})") } else { format!("min({d1}, {d2})") }
            }
            SdfOp::Subtract { a, b, smooth } => {
                let d1 = self.emit_expression(a, p_var);
                let d2 = self.emit_expression(b, p_var);
                if *smooth > 0.0 { format!("op_smooth_subtraction({d1}, {d2}, {smooth:.4})") } else { format!("max({d1}, -{d2})") }
            }
            SdfOp::Intersect { a, b, smooth: _ } => {
                let d1 = self.emit_expression(a, p_var);
                let d2 = self.emit_expression(b, p_var);
                format!("max({d1}, {d2})")
            }
            SdfOp::Translate { target, offset } => {
                let new_p = format!("({p_var} - vec3<f32>({:.4}, {:.4}, {:.4}))", offset[0], offset[1], offset[2]);
                self.emit_expression(target, &new_p)
            }
            SdfOp::Rotate { target, axis, angle_deg } => {
                let rad = (-angle_deg).to_radians(); // Inverse
                let axis_name = if axis[0] > 0.9 { "x" } else if axis[1] > 0.9 { "y" } else { "z" };
                let new_p = format!("rotate_{axis_name}({p_var}, {rad:.4})");
                self.emit_expression(target, &new_p)
            }
        }
    }
}