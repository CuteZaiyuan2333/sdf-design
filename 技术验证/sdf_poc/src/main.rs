mod sdf_widget;

use eframe::{egui, wgpu};
use std::sync::Arc;
use sdf_widget::{SdfRenderResources, sdf_view};

struct SdfApp {
    sdf_resources: Option<Arc<SdfRenderResources>>,
}

impl SdfApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts...

        let sdf_resources = SdfRenderResources::new(cc).map(Arc::new);
        
        if sdf_resources.is_none() {
            log::error!("WGPU Render State not available.");
        }

        Self {
            sdf_resources,
        }
    }
}

impl eframe::App for SdfApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Continuous repaint for the animation
        ctx.request_repaint();

        egui::SidePanel::left("settings_panel").show(ctx, |ui| {
            ui.heading("SDF Design POC");
            ui.separator();
            ui.label("Pure Rust + WGPU + SDF");
            ui.separator();
            ui.label("This component renders a Raymarched scene inside an egui area.");
            
            ui.add_space(20.0);
            ui.label("Controls (Placeholder):");
            let mut val = 0.0;
            ui.add(egui::Slider::new(&mut val, 0.0..=1.0).text("Radius"));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(resources) = &self.sdf_resources {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    sdf_view(ui, resources);
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Failed to load WGPU resources.");
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SDF Design Studio (Rust)",
        options,
        Box::new(|cc| Ok(Box::new(SdfApp::new(cc)))),
    )
}