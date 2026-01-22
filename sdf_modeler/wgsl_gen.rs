use super::sdf_ast::{SdfNode, SdfOp};

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
            "struct SdfResult {{
                dist: f32,
                color: vec3<f32>,
            }}

            fn map(p_in: vec3<f32>) -> SdfResult {{
                return {};
            }}",
            expression
        )
    }

    fn emit_expression(&self, node: &SdfNode, p_var: &str) -> String {
        match &node.op {
            SdfOp::Sphere { radius } => format!("SdfResult(sd_sphere({p_var}, {radius:.4}), vec3<f32>(0.2, 0.55, 1.0))"),
            SdfOp::Box { size } => format!("SdfResult(sd_box({p_var}, vec3<f32>({:.4}, {:.4}, {:.4})), vec3<f32>(0.2, 0.55, 1.0))", size[0], size[1], size[2]),
            SdfOp::Cylinder { radius, height } => format!("SdfResult(sd_cylinder({p_var}, {radius:.4}, {height:.4}), vec3<f32>(0.2, 0.55, 1.0))"),
            SdfOp::Torus { major_radius, minor_radius } => format!("SdfResult(sd_torus({p_var}, vec2<f32>({major_radius:.4}, {minor_radius:.4})), vec3<f32>(0.2, 0.55, 1.0))"),
            
            SdfOp::Union { a, b, smooth } => {
                let res1 = self.emit_expression(a, p_var);
                let res2 = self.emit_expression(b, p_var);
                if *smooth > 0.0 {
                    format!("op_union_smooth({res1}, {res2}, {smooth:.4})")
                } else {
                    format!("op_union({res1}, {res2})")
                }
            }
            SdfOp::Subtract { a, b, smooth } => {
                let res1 = self.emit_expression(a, p_var);
                let res2 = self.emit_expression(b, p_var);
                if *smooth > 0.0 {
                    format!("op_subtract_smooth({res1}, {res2}, {smooth:.4})")
                } else {
                    format!("op_subtract({res1}, {res2})")
                }
            }
            SdfOp::Intersect { a, b, smooth: _ } => {
                let res1 = self.emit_expression(a, p_var);
                let res2 = self.emit_expression(b, p_var);
                format!("op_intersect({res1}, {res2})")
            }
            SdfOp::Translate { target, offset } => {
                let new_p = format!("({p_var} - vec3<f32>({:.4}, {:.4}, {:.4}))", offset[0], offset[1], offset[2]);
                self.emit_expression(target, &new_p)
            }
            SdfOp::Rotate { target, axis, angle_deg } => {
                let rad = (-angle_deg).to_radians();
                let axis_name = if axis[0] > 0.9 { "x" } else if axis[1] > 0.9 { "y" } else { "z" };
                let new_p = format!("rotate_{axis_name}({p_var}, {rad:.4})");
                self.emit_expression(target, &new_p)
            }
            SdfOp::Mirror { target, axis } => {
                // p' = abs(p) for the mirror axis
                let mut p_parts = [format!("{p_var}.x"), format!("{p_var}.y"), format!("{p_var}.z")];
                if axis[0] > 0.9 { p_parts[0] = format!("abs({})", p_parts[0]); }
                if axis[1] > 0.9 { p_parts[1] = format!("abs({})", p_parts[1]); }
                if axis[2] > 0.9 { p_parts[2] = format!("abs({})", p_parts[2]); }
                let new_p = format!("vec3<f32>({}, {}, {})", p_parts[0], p_parts[1], p_parts[2]);
                self.emit_expression(target, &new_p)
            }
            SdfOp::Color { target, color } => {
                let mut res = self.emit_expression(target, p_var);
                // We wrap the expression and just replace the color field
                format!("set_color({}, vec3<f32>({:.4}, {:.4}, {:.4}))", res, color[0], color[1], color[2])
            }
        }
    }
}
