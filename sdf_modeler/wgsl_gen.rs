use super::sdf_ast::{SdfNode, SdfOp, SsaaLevel};

pub struct WgslGenerator;

impl WgslGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&mut self, root: &SdfNode, ssaa: SsaaLevel) -> String {
        let map_expr = self.emit_expression(root, "p_in");
        let n = ssaa.to_u32();
        
        let fs_main = if n <= 1 {
            // No SSAA
            format!(
                "@fragment
                fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
                    let pixel_pos = in.clip_position.xy;
                    let rect_min = uniforms.rect_data.xy;
                    let rect_size = uniforms.rect_data.zw;
                    let aspect = rect_size.x / rect_size.y;
                    let uv = (((pixel_pos - rect_min) / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, -1.0);
                    return vec4<f32>(render_scene(uv), 1.0);
                }}"
            )
        } else {
            // Dynamic SSAA loop
            format!(
                "@fragment
                fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
                    let pixel_pos = in.clip_position.xy;
                    let rect_min = uniforms.rect_data.xy;
                    let rect_size = uniforms.rect_data.zw;
                    let aspect = rect_size.x / rect_size.y;
                    var total_color = vec3<f32>(0.0);
                    let n: f32 = {n}.0;
                    for (var iy: i32 = 0; iy < {n}; iy = iy + 1) {{
                        for (var ix: i32 = 0; ix < {n}; ix = ix + 1) {{
                            let offset = (vec2<f32>(f32(ix), f32(iy)) + 0.5) / n - 0.5;
                            let uv = (((pixel_pos + offset - rect_min) / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, -1.0);
                            total_color += render_scene(uv);
                        }}
                    }}
                    return vec4<f32>(total_color / (n * n), 1.0);
                }}",
                n = n
            )
        };

        format!(
            "struct SdfResult {{
                dist: f32,
                color: vec3<f32>,
            }}

            fn map(p_in: vec3<f32>) -> SdfResult {{
                return {};
            }}
            
            {}
            ",
            map_expr, fs_main
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
                let mut p_parts = [format!("{p_var}.x"), format!("{p_var}.y"), format!("{p_var}.z")];
                if axis[0] > 0.9 { p_parts[0] = format!("abs({})", p_parts[0]); }
                if axis[1] > 0.9 { p_parts[1] = format!("abs({})", p_parts[1]); }
                if axis[2] > 0.9 { p_parts[2] = format!("abs({})", p_parts[2]); }
                let new_p = format!("vec3<f32>({}, {}, {})", p_parts[0], p_parts[1], p_parts[2]);
                self.emit_expression(target, &new_p)
            }
            SdfOp::Color { target, color } => {
                let res = self.emit_expression(target, p_var);
                format!("set_color({}, vec3<f32>({:.4}, {:.4}, {:.4}))", res, color[0], color[1], color[2])
            }
        }
    }
}
