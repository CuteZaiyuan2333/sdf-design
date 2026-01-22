use eframe::egui_wgpu::{self, CallbackTrait};
use eframe::egui::{self, Sense, Ui, Rect, Vec2};
use eframe::wgpu;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Uniforms {
    rect_data: [f32; 4],     // x, y, w, h
    time_data: [f32; 4],     // time, padding...
    cam_pos:   [f32; 4],     // x, y, z, padding
    cam_right: [f32; 4],     // x, y, z, padding
    cam_up:    [f32; 4],     // x, y, z, padding
    cam_front: [f32; 4],     // x, y, z, padding
}

pub struct SdfRenderResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    start_time: std::time::Instant,
}

impl SdfRenderResources {
    pub fn create(device: &wgpu::Device, target_format: wgpu::TextureFormat, shader_source: &str) -> Option<Self> {
        // Compile Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SDF Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Initial zeroed uniforms
        let uniforms = Uniforms {
            rect_data: [0.0; 4],
            time_data: [0.0; 4],
            cam_pos:   [0.0; 4],
            cam_right: [0.0; 4],
            cam_up:    [0.0; 4],
            cam_front: [0.0; 4],
        };
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SDF Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SDF Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SDF Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SDF Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[], 
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Some(Self {
            pipeline,
            bind_group,
            uniform_buffer,
            start_time: std::time::Instant::now(),
        })
    }

    pub fn new(cc: &eframe::CreationContext<'_>, shader_source: &str) -> Option<Self> {
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;
        Self::create(&wgpu_render_state.device, wgpu_render_state.target_format, shader_source)
    }
    
    pub fn from_wgpu_state(rs: &egui_wgpu::RenderState, shader_source: &str) -> Option<Self> {
        Self::create(&rs.device, rs.target_format, shader_source)
    }
}

pub struct CameraUniformData {
    pub pos: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub front: [f32; 3],
}

pub struct SdfCallback {
    resources: Arc<SdfRenderResources>,
    time: f32,
    rect: Rect,
    camera: CameraUniformData,
}

impl CallbackTrait for SdfCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let ppp = screen_descriptor.pixels_per_point;
        
        let c = &self.camera;
        
        let uniforms = Uniforms {
            rect_data: [
                self.rect.min.x * ppp,
                self.rect.min.y * ppp,
                self.rect.width() * ppp,
                self.rect.height() * ppp,
            ],
            time_data: [self.time, 0.0, 0.0, 0.0],
            cam_pos:   [c.pos[0], c.pos[1], c.pos[2], 1.0],
            cam_right: [c.right[0], c.right[1], c.right[2], 0.0],
            cam_up:    [c.up[0], c.up[1], c.up[2], 0.0],
            cam_front: [c.front[0], c.front[1], c.front[2], 0.0],
        };
        
        queue.write_buffer(&self.resources.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        render_pass.set_pipeline(&self.resources.pipeline);
        render_pass.set_bind_group(0, &self.resources.bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }
}

pub fn sdf_view(ui: &mut Ui, resources: &Arc<SdfRenderResources>, camera: CameraUniformData) -> eframe::egui::Response {
    let available = ui.available_size();
    let size = Vec2::new(available.x.max(100.0), available.y.max(100.0));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    
    let time = resources.start_time.elapsed().as_secs_f32();

    let callback = egui_wgpu::Callback::new_paint_callback(
        rect,
        SdfCallback {
            resources: resources.clone(),
            time,
            rect,
            camera,
        },
    );

    ui.painter().add(callback);
    
    response
}