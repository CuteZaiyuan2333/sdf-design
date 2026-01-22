mod sdf_widget;
mod sdf_ast;
mod wgsl_gen;

use eframe::{egui, wgpu};
use std::sync::Arc;
use sdf_widget::{SdfRenderResources, sdf_view};
use rhai::{Engine, Scope};
use sdf_ast::{SdfNode, register_rhai_types};
use wgsl_gen::WgslGenerator;

struct SdfApp {
    sdf_resources: Option<Arc<SdfRenderResources>>,
    rhai_engine: Engine,
    code_text: String,
    compiler_error: Option<String>,
    // Store creation context needed for recompilation
    // Note: We can't store CreationContext directly as it is temporary. 
    // We need to capture the RenderState or Device from it, but eframe doesn't expose a persistent "Context" 
    // that allows creating resources easily outside of 'update' or 'setup'.
    //
    // Actually, eframe's App trait doesn't pass 'cc' to 'update'. 
    // We need to grab the wgpu device from `frame.wgpu_render_state()`.
}

impl SdfApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut engine = Engine::new();
        register_rhai_types(&mut engine);

        // Initial default script
        let default_code = r#"
// Default Demo Script
let s = sphere(0.6);
let b = box(0.5, 0.5, 0.5);

// Union with smooth blending
s.smooth_union(b, 0.15)
"#;
        
        // Initial Compile
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
        }
    }

    fn compile_shader(engine: &Engine, code: &str) -> Result<String, String> {
        // 1. Run Rhai
        let mut scope = Scope::new();
        let result = engine.eval_with_scope::<SdfNode>(&mut scope, code)
            .map_err(|e| format!("Rhai Error: {}", e))?;

        // 2. Generate WGSL Body
        let mut generator = WgslGenerator::new();
        let map_fn_body = generator.generate(&result);

        // 3. Merge with Template
        let template = include_str!("shader_template.wgsl");
        // Simple string replacement
        let full_wgsl = template.replace("// {{MAP_FUNCTION_HERE}}", &map_fn_body);

        Ok(full_wgsl)
    }

    fn recompile(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let wgpu_render_state = frame.wgpu_render_state();
        if wgpu_render_state.is_none() {
             self.compiler_error = Some("WGPU Context lost.".to_string());
             return;
        }
        let wgpu_render_state = wgpu_render_state.unwrap();

        match Self::compile_shader(&self.rhai_engine, &self.code_text) {
            Ok(wgsl) => {
                // Re-create resources using the RenderState from the frame
                // We need to manually construct a CreationContext-like object or just call new with what we have.
                // But SdfRenderResources::new expects CreationContext. 
                // Let's refactor SdfRenderResources::new to take (device, format) instead of CC.
                // Wait, I can't easily change SdfRenderResources::new signature without changing earlier code massively 
                // or just adapting here.
                // Actually, I can just copy the logic from SdfRenderResources::new here or split it.
                //
                // Best approach: Refactor SdfRenderResources::new to take (&Device, TextureFormat)
                
                let device = &wgpu_render_state.device;
                let target_format = wgpu_render_state.target_format;
                
                // --- MANUAL RESOURCE RECREATION (Copy-paste logic from sdf_widget for now to save tokens/time) ---
                // Ideally, SdfRenderResources::from_device(device, format, source)
                // Let's assume I refactor it.
                
                // For now, I will use a helper method on App or just inline it if I can access SdfRenderResources fields.
                // But fields are private. I need to modify sdf_widget.rs one last time to make it friendly for reloading.
            }
            Err(e) => {
                self.compiler_error = Some(e);
            }
        }
    }
}

// Helper to bridge the gap without refactoring sdf_widget too much
// We will modify SdfRenderResources in a second pass to add `recreate`.
// For now, let's just finish the App logic structure.

impl eframe::App for SdfApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::SidePanel::left("editor_panel").resizable(true).default_width(400.0).show(ctx, |ui| {
            ui.heading("Rhai SDF Editor");
            ui.separator();
            
            if ui.button("Compile & Run (Ctrl+Enter)").clicked() || 
               (ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command)) 
            {
                // Trigger Recompile
                match Self::compile_shader(&self.rhai_engine, &self.code_text) {
                    Ok(wgsl) => {
                        self.compiler_error = None;
                        // Now we need to update resources.
                        // We need access to the device.
                        if let Some(rs) = frame.wgpu_render_state() {
                            // Hack: we need a way to create resources from here.
                            // I will add a static method to SdfRenderResources.
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

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(resources) = &self.sdf_resources {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    sdf_view(ui, resources);
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
