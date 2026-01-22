use egui::{Ui, WidgetText};
use std::sync::Arc;
use crate::{Plugin, AppCommand, TabInstance, Tab};

// 导入原有的逻辑模块
mod sdf_ast;
mod sdf_widget;
mod wgsl_gen;

use sdf_widget::{sdf_view, CameraUniformData};
use sdf_ast::{SdfNode, register_rhai_types};
use wgsl_gen::WgslGenerator;
use glam::Vec3;
use rhai::{Engine, Scope};

// --- Camera 逻辑保持不变 ---

struct Camera {
    pos: Vec3,
    yaw: f32,   
    pitch: f32, 
}

impl Default for Camera {
    fn default() -> Self {
        let pos = Vec3::new(5.0, 5.0, 5.0);
        let dir = -pos.normalize();
        let yaw = dir.z.atan2(dir.x);
        let pitch = dir.y.asin();
        Self { pos, yaw, pitch }
    }
}

impl Camera {
    fn update(&mut self, ui: &mut egui::Ui, response: &egui::Response) {
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            let sensitivity = 0.005;
            self.yaw += delta.x * sensitivity;
            self.pitch -= delta.y * sensitivity;
            self.pitch = self.pitch.clamp(-1.5, 1.5);
        }

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
                if i.key_down(egui::Key::E) { move_dir += up; }
                if i.key_down(egui::Key::Q) { move_dir -= up; }
                
                if move_dir.length_squared() > 0.0 {
                    self.pos += move_dir.normalize() * speed;
                }
            });
        }
    }
}

// --- Tab 实现 ---

#[derive(Clone)]
pub struct SdfTab {
    sdf_resources: Arc<parking_lot::RwLock<Option<Arc<sdf_widget::SdfRenderResources>>>>,
    rhai_engine: Arc<Engine>,
    code_text: String,
    current_shader: String,
    compiler_error: Option<String>,
    camera: Arc<std::sync::Mutex<Camera>>,
}

impl std::fmt::Debug for SdfTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdfTab").field("code", &self.code_text).finish()
    }
}

impl SdfTab {
    fn new() -> Self {
        let mut engine = Engine::new();
        register_rhai_types(&mut engine);

        let default_code = r#"
// Colors and Mirroring demo
let body = box(1.0, 0.2, 0.5).color(0.8, 0.8, 0.8);
let wheel = torus(0.4, 0.1).rotate_x(90.0).color(0.2, 0.2, 0.2);

// Move wheel to position and mirror it across X and Z axes
let wheels = wheel.translate(1.0, 0.0, 0.6).mirror_x().mirror_z();

body.union(wheels)
"#;
        
        let initial_shader = Self::compile_shader(&engine, default_code).unwrap_or_default();

        Self {
            sdf_resources: Arc::new(parking_lot::RwLock::new(None)),
            rhai_engine: Arc::new(engine),
            code_text: default_code.to_string(),
            current_shader: initial_shader,
            compiler_error: None,
            camera: Arc::new(std::sync::Mutex::new(Camera::default())),
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

impl TabInstance for SdfTab {
    fn title(&self) -> WidgetText {
        "SDF Modeler".into()
    }

    fn ui(&mut self, ui: &mut Ui, control: &mut Vec<AppCommand>) {
        ui.ctx().request_repaint();

        egui::SidePanel::left("sdf_editor_panel").resizable(true).show_inside(ui, |ui| {
            ui.heading("Rhai SDF Editor");
            ui.separator();
            
            let compile_requested = ui.button("🔨 Compile & Run (Ctrl+Enter)").clicked() || 
                                   (ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command));

            if compile_requested {
                match Self::compile_shader(&self.rhai_engine, &self.code_text) {
                    Ok(wgsl) => {
                        self.compiler_error = None;
                        self.current_shader = wgsl;
                        // 重置资源，触发下一帧重新创建
                        *self.sdf_resources.write() = None;
                        control.push(AppCommand::Notify { 
                            message: "Shader compiled successfully".into(), 
                            level: crate::NotificationLevel::Success 
                        });
                    }
                    Err(e) => {
                        self.compiler_error = Some(e);
                    }
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

        egui::CentralPanel::default().show_inside(ui, |ui| {
             let mut camera = self.camera.lock().unwrap();
             
             let front = Vec3::new(
                camera.yaw.cos() * camera.pitch.cos(),
                camera.pitch.sin(),
                camera.yaw.sin() * camera.pitch.cos()
            ).normalize();

            let global_up = Vec3::new(0.0, 1.0, 0.0);
            let right = front.cross(global_up).normalize();
            let up = right.cross(front).normalize();

            let cam_data = CameraUniformData {
                pos: camera.pos.into(),
                front: front.into(),
                right: right.into(),
                up: up.into(),
            };
            
            // 直接调用 sdf_view，它内部会处理资源的延迟创建
            let response = sdf_view(ui, &self.sdf_resources, self.current_shader.clone(), cam_data);
            camera.update(ui, &response);
        });
    }

    fn box_clone(&self) -> Box<dyn TabInstance> {
        Box::new(self.clone())
    }
}

// --- Plugin 实现 ---

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn name(&self) -> &str {
        crate::plugins::PLUGIN_NAME_SDF_MODELER
    }

    fn on_menu_bar(&mut self, ui: &mut Ui, control: &mut Vec<AppCommand>) {
        if ui.button("💠 SDF Modeler").clicked() {
            control.push(AppCommand::OpenTab(Tab::new(Box::new(SdfTab::new()))));
        }
    }
}

pub fn create() -> SdfPlugin {
    SdfPlugin
}