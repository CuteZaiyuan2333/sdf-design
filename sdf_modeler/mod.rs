use egui::{Ui, WidgetText};
use std::sync::Arc;
use std::path::PathBuf;
use std::fs;
use crate::{Plugin, AppCommand, TabInstance, Tab};

// Import internal modules
mod sdf_ast;
mod sdf_widget;
mod wgsl_gen;

use sdf_widget::{sdf_view, CameraUniformData};
use sdf_ast::{SdfNode, register_rhai_types};
use wgsl_gen::WgslGenerator;
use glam::Vec3;
use rhai::{Engine, Scope};

// --- Camera Logic (Unchanged) ---

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
        
        // Mouse Look
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            let sensitivity = 0.005;
            self.yaw += delta.x * sensitivity;
            self.pitch -= delta.y * sensitivity;
            self.pitch = self.pitch.clamp(-1.5, 1.5);
        }

        // Keyboard Move
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

// --- Tab Implementation ---

#[derive(Clone)]
pub struct SdfTab {
    // 3D Resources
    sdf_resources: Arc<parking_lot::RwLock<Option<Arc<sdf_widget::SdfRenderResources>>>>,
    camera: Arc<std::sync::Mutex<Camera>>,
    current_shader: String,
    
    // Logic Resources
    rhai_engine: Arc<Engine>,
    
    // Project State
    project_path: Option<PathBuf>,
    compiler_error: Option<String>,
}

impl std::fmt::Debug for SdfTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdfTab")
         .field("project_path", &self.project_path)
         .finish()
    }
}

impl SdfTab {
    fn new() -> Self {
        let mut engine = Engine::new();
        register_rhai_types(&mut engine);

        Self {
            sdf_resources: Arc::new(parking_lot::RwLock::new(None)),
            camera: Arc::new(std::sync::Mutex::new(Camera::default())),
            current_shader: String::new(),
            
            rhai_engine: Arc::new(engine),
            
            project_path: None,
            compiler_error: None,
        }
    }

    fn compile_project(&mut self) -> Result<String, String> {
        let path = self.project_path.as_ref().ok_or("No project opened")?;
        let entry_file = path.join("main.rhai");
        
        let code = fs::read_to_string(&entry_file)
            .map_err(|e| format!("Failed to read main.rhai: {}", e))?;

        let mut scope = Scope::new();
        let result = self.rhai_engine.eval_with_scope::<SdfNode>(&mut scope, &code)
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
        if let Some(path) = &self.project_path {
            if let Some(name) = path.file_name() {
                return format!("SDF: {}", name.to_string_lossy()).into();
            }
        }
        "SDF Modeler".into()
    }

    fn ui(&mut self, ui: &mut Ui, control: &mut Vec<AppCommand>) {
        // --- Top Bar: Project Controls ---
        egui::TopBottomPanel::top("sdf_top_bar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Open Project...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.project_path = Some(path);
                        // Auto-compile on open
                        match self.compile_project() {
                            Ok(wgsl) => {
                                self.compiler_error = None;
                                self.current_shader = wgsl;
                                *self.sdf_resources.write() = None;
                                control.push(AppCommand::Notify { 
                                    message: "Project opened & compiled".into(), 
                                    level: crate::NotificationLevel::Success 
                                });
                            }
                            Err(e) => {
                                self.compiler_error = Some(e);
                                control.push(AppCommand::Notify { 
                                    message: "Project opened but compilation failed".into(), 
                                    level: crate::NotificationLevel::Warning 
                                });
                            }
                        }
                    }
                }

                if let Some(path) = &self.project_path {
                    ui.label(path.to_string_lossy().to_string());
                    ui.separator();
                    
                    let run_btn = ui.button("▶ Compile & Run");
                    if run_btn.clicked() {
                        match self.compile_project() {
                            Ok(wgsl) => {
                                self.compiler_error = None;
                                self.current_shader = wgsl;
                                *self.sdf_resources.write() = None;
                                control.push(AppCommand::Notify { 
                                    message: "Project compiled".into(), 
                                    level: crate::NotificationLevel::Success 
                                });
                            }
                            Err(e) => {
                                self.compiler_error = Some(e);
                                control.push(AppCommand::Notify { 
                                    message: "Compilation failed".into(), 
                                    level: crate::NotificationLevel::Error 
                                });
                            }
                        }
                    }
                } else {
                    ui.label("No project selected.");
                    ui.weak("Select a folder containing 'main.rhai'");
                }
            });
            
            // Error Display
            if let Some(err) = &self.compiler_error {
                ui.separator();
                ui.colored_label(egui::Color32::RED, err);
            }
        });

        // --- Central: 3D Viewport ---
        egui::CentralPanel::default().show_inside(ui, |ui| {
             if self.current_shader.is_empty() {
                 ui.centered_and_justified(|ui| {
                     if self.project_path.is_none() {
                         ui.label("Open a project to start.");
                     } else {
                         ui.label("Ready to compile.");
                     }
                 });
                 return;
             }

             let mut camera = self.camera.lock().unwrap();
             
             // Calculate Camera Basis
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
            
            // Draw 3D View
            let response = sdf_view(ui, &self.sdf_resources, self.current_shader.clone(), cam_data);
            
            // Handle Camera Input
            camera.update(ui, &response);
            
            // Overlay: Status
            let rect = response.rect;
            ui.put(
                egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(10.0, -30.0), egui::vec2(200.0, 20.0)),
                |ui: &mut Ui| {
                    ui.colored_label(egui::Color32::WHITE, format!("Cam: [{:.1}, {:.1}, {:.1}]", camera.pos.x, camera.pos.y, camera.pos.z))
                }
            );
        });
    }

    fn box_clone(&self) -> Box<dyn TabInstance> {
        Box::new(self.clone())
    }
}

// --- Plugin Entry ---

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
