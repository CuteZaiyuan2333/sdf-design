mod sdf_widget;
mod sdf_ast;
mod wgsl_gen;

use eframe::{egui, wgpu};
use std::sync::Arc;
use sdf_widget::{SdfRenderResources, sdf_view, CameraUniformData};
use rhai::{Engine, Scope};
use sdf_ast::{SdfNode, register_rhai_types};
use wgsl_gen::WgslGenerator;
use glam::{Vec3, Mat3}; 

struct Camera {
    pos: Vec3,
    yaw: f32,   
    pitch: f32, 
}

impl Default for Camera {
    fn default() -> Self {
        // Start at (5, 5, 5)
        let pos = Vec3::new(5.0, 5.0, 5.0);
        // Look at (0, 0, 0)
        // dir = (0,0,0) - (5,5,5) = (-5, -5, -5)
        let dir = -pos.normalize();
        
        let yaw = dir.z.atan2(dir.x); // atan2(-5, -5)
        let pitch = dir.y.asin();      // asin(-5 / sqrt(75))
        
        Self {
            pos,
            yaw,
            pitch,
        }
    }
}

impl Camera {
    fn update(&mut self, ui: &mut egui::Ui, response: &egui::Response) {
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            let sensitivity = 0.005;
            
            self.yaw += delta.x * sensitivity;
            self.pitch += delta.y * sensitivity;
            self.pitch = self.pitch.clamp(-1.5, 1.5);
        }

        // Standard movement
        let forward = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin()).normalize();
        let right = Vec3::new(-self.yaw.sin(), 0.0, self.yaw.cos()).normalize();
        let up = Vec3::new(0.0, 1.0, 0.0);
        let speed = 4.0 * dt; 
        
        if response.hovered() || response.dragged() {
            ui.input(|i| {
                let mut move_dir = Vec3::ZERO;
                if i.key_down(egui::Key::W) { move_dir += forward; }
                if i.key_down(egui::Key::S) { move_dir -= forward; }
                if i.key_down(egui::Key::A) { move_dir -= right; }
                if i.key_down(egui::Key::D) { move_dir += right; }
                if i.key_down(egui::Key::E) { move_dir += up; }    // E: 上升
                if i.key_down(egui::Key::Q) { move_dir -= up; }    // Q: 下降
                
                if move_dir.length_squared() > 0.0 {
                    self.pos += move_dir.normalize() * speed;
                }
            });
        }
    }
}

struct SdfApp {
    sdf_resources: Option<Arc<SdfRenderResources>>,
    rhai_engine: Engine,
    code_text: String,
    compiler_error: Option<String>,
    camera: Camera,
}

impl SdfApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut engine = Engine::new();
        register_rhai_types(&mut engine);

        let default_code = r#"
// New Primitives: cylinder(radius, height), torus(major, minor)
// Try this: A pipe with a hole
let pipe = cylinder(0.5, 2.0);
let hole = cylinder(0.4, 2.1);
pipe.subtract(hole)
"#;
        
        let initial_shader = Self::compile_shader(&engine, default_code);
        let sdf_resources = match initial_shader {
            Ok(wgsl) => SdfRenderResources::new(cc, &wgsl).map(Arc::new),
            Err(e) => {
                println!("Initial compile error: {}", e);
                None
            }
        };

        Self {
            sdf_resources,
            rhai_engine: engine,
            code_text: default_code.to_string(),
            compiler_error: None,
            camera: Camera::default(),
        }
    }

    fn compile_shader(engine: &Engine, code: &str) -> Result<String, String> {
        let mut scope = Scope::new();
        let result = engine.eval_with_scope::<SdfNode>(&mut scope, code)
            .map_err(|e| format!("Rhai Error: {}", e))?;

        let mut generator = WgslGenerator::new();
        let map_fn_body = generator.generate(&result);

        let template = include_str!("shader_template.wgsl");
        let full_wgsl = template.replace("// {{MAP_FUNCTION_HERE}}", &map_fn_body);

        Ok(full_wgsl)
    }
}

impl eframe::App for SdfApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint(); 

        egui::SidePanel::left("editor_panel").resizable(true).default_width(400.0).show(ctx, |ui| {
            ui.heading("Rhai SDF Editor");
            ui.label("Controls:");
            ui.label("- Drag Middle Mouse: Rotate Look");
            ui.label("- W/A/S/D: Move Horizontal");
            ui.label("- Q/E: Move Down/Up");
            ui.separator();
            
            if ui.button("Compile & Run (Ctrl+Enter)").clicked() || 
               (ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command)) 
            {
                match Self::compile_shader(&self.rhai_engine, &self.code_text) {
                    Ok(wgsl) => {
                        self.compiler_error = None;
                        if let Some(rs) = frame.wgpu_render_state() {
                            if let Some(new_res) = SdfRenderResources::from_wgpu_state(&rs, &wgsl) {
                                self.sdf_resources = Some(Arc::new(new_res));
                            } else {
                                self.compiler_error = Some("Failed to create WGPU resources".to_string());
                            }
                        }
                    }
                    Err(e) => self.compiler_error = Some(e),
                }
            }

            if let Some(err) = &self.compiler_error {
                ui.colored_label(egui::Color32::RED, err);
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.code_text)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(30)
                );
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Camera Pos: [{:.2}, {:.2}, {:.2}]", self.camera.pos.x, self.camera.pos.y, self.camera.pos.z));
                ui.separator();
                ui.label(format!("Yaw: {:.1}°, Pitch: {:.1}°", self.camera.yaw.to_degrees(), self.camera.pitch.to_degrees()));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(resources) = &self.sdf_resources.clone() {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    
                    let front = Vec3::new(
                        self.camera.yaw.cos() * self.camera.pitch.cos(),
                        self.camera.pitch.sin(),
                        self.camera.yaw.sin() * self.camera.pitch.cos()
                    ).normalize();

                    let global_up = Vec3::new(0.0, 1.0, 0.0);
                    let right = front.cross(global_up).normalize();
                    let up = right.cross(front).normalize();

                    let cam_data = CameraUniformData {
                        pos: self.camera.pos.into(),
                        front: front.into(),
                        right: right.into(),
                        up: up.into(),
                    };
                    
                    let response = sdf_view(ui, resources, cam_data);
                    self.camera.update(ui, &response);
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No Shader Compiled.");
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "SDF Rhai Modeler",
        options,
        Box::new(|cc| Ok(Box::new(SdfApp::new(cc)))),
    )
}