struct Uniforms {
    rect_data: vec4<f32>,
    time_data: vec4<f32>,
    cam_pos: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    cam_front: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// --- SDF Primitives ---

fn sd_sphere(p: vec3<f32>, s: f32) -> f32 {
    return length(p) - s;
}

fn sd_box(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sd_cylinder(p: vec3<f32>, r: f32, h: f32) -> f32 {
    let d = abs(vec2<f32>(length(p.xz), p.y)) - vec2<f32>(r, h);
    return min(max(d.x, d.y), 0.0) + length(max(d, vec3<f32>(0.0).xy));
}

fn sd_torus(p: vec3<f32>, t: vec2<f32>) -> f32 {
    let q = vec2<f32>(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

// --- SDF Operations ---

fn op_smooth_union(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

fn op_smooth_subtraction(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (d2 + d1) / k, 0.0, 1.0);
    return mix(d1, -d2, h) + k * h * (1.0 - h);
}

// --- Transforms ---

fn rotate_x(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle); let s = sin(angle);
    return vec3<f32>(p.x, c * p.y - s * p.z, s * p.y + c * p.z);
}

fn rotate_y(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle); let s = sin(angle);
    return vec3<f32>(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}

fn rotate_z(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle); let s = sin(angle);
    return vec3<f32>(c * p.x - s * p.y, s * p.x + c * p.y, p.z);
}

// {{MAP_FUNCTION_HERE}}

fn calc_normal(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.0005, 0.0);
    return normalize(vec3<f32>(
        map(p + e.xyy) - map(p - e.xyy),
        map(p + e.yxy) - map(p - e.yxy),
        map(p + e.yyx) - map(p - e.yyx)
    ));
}

fn ray_march(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    var t = 0.0;
    for (var i = 0; i < 128; i++) {
        let p = ro + rd * t;
        let d = map(p);
        if (d < 0.0005 || t > 50.0) { break; }
        t += d;
    }
    return t;
}

// Infinite Grid Logic
fn get_grid_color(p: vec3<f32>, rd: vec3<f32>) -> vec4<f32> {
    let t = -p.y / rd.y;
    if (t > 0.0 && t < 100.0) {
        let pos = p + rd * t;
        let grid = abs(fract(pos.xz - 0.5) - 0.5) / fwidth(pos.xz);
        let line = min(grid.x, grid.y);
        let color = 1.0 - min(line, 1.0);
        let alpha = color * exp(-t * 0.05) * 0.3;
        return vec4<f32>(vec3<f32>(0.5), alpha);
    }
    return vec4<f32>(0.0);
}

fn render_scene(uv: vec2<f32>) -> vec3<f32> {
    let ro = uniforms.cam_pos.xyz;
    let forward = normalize(uniforms.cam_front.xyz);
    let right = normalize(uniforms.cam_right.xyz);
    let up = normalize(uniforms.cam_up.xyz);
    let rd = normalize(uv.x * right + uv.y * up + 1.8 * forward);

    let t = ray_march(ro, rd);
    let bg_color = vec3<f32>(0.08, 0.08, 0.1);
    
    // Background and Grid
    var col = bg_color;
    let grid = get_grid_color(ro, rd);
    col = mix(col, grid.rgb, grid.a);

    if (t < 50.0) {
        let p = ro + rd * t;
        let normal = calc_normal(p);
        let light_dir = normalize(vec3<f32>(2.0, 4.0, 3.0) - p);
        let diff = max(dot(normal, light_dir), 0.0);
        let view_dir = normalize(ro - p);
        let reflect_dir = reflect(-light_dir, normal);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
        let fresnel = pow(1.0 - max(dot(normal, view_dir), 0.0), 5.0) * 0.3;
        
        let base_col = vec3<f32>(0.2, 0.55, 1.0);
        let lit_col = base_col * (diff + 0.1) + vec3<f32>(spec * 0.4) + vec3<f32>(fresnel);
        col = lit_col;
    }
    
    return col;
}

struct VertexOutput { @builtin(position) clip_position: vec4<f32> };

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    let x = f32(i32(idx) & 1) * 4.0 - 1.0;
    let y = f32(i32(idx) & 2) * 2.0 - 1.0;
    return VertexOutput(vec4<f32>(x, y, 0.0, 1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_pos = in.clip_position.xy;
    let rect_min = uniforms.rect_data.xy;
    let rect_size = uniforms.rect_data.zw;
    let aspect = rect_size.x / rect_size.y;
    
    var total = vec3<f32>(0.0);
    
    // Sample 1
    var uv = (((pixel_pos + vec2<f32>(-0.25, -0.25) - rect_min) / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, 1.0);
    total += render_scene(uv);
    
    // Sample 2
    uv = (((pixel_pos + vec2<f32>(0.25, -0.25) - rect_min) / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, 1.0);
    total += render_scene(uv);
    
    // Sample 3
    uv = (((pixel_pos + vec2<f32>(-0.25, 0.25) - rect_min) / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, 1.0);
    total += render_scene(uv);
    
    // Sample 4
    uv = (((pixel_pos + vec2<f32>(0.25, 0.25) - rect_min) / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, 1.0);
    total += render_scene(uv);

    return vec4<f32>(total / 4.0, 1.0);
}
