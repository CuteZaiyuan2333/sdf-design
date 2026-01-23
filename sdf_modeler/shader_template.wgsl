struct Uniforms {
    rect_data: vec4<f32>,
    time_data: vec4<f32>,
    cam_pos: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up:    vec4<f32>,
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

// --- Result & Material Helpers ---

fn op_union(a: SdfResult, b: SdfResult) -> SdfResult {
    if (a.dist < b.dist) { return a; }
    return b;
}

fn op_union_smooth(a: SdfResult, b: SdfResult, k: f32) -> SdfResult {
    let h = clamp(0.5 + 0.5 * (b.dist - a.dist) / k, 0.0, 1.0);
    let d = mix(b.dist, a.dist, h) - k * h * (1.0 - h);
    let col = mix(b.color, a.color, h);
    return SdfResult(d, col);
}

fn op_subtract(a: SdfResult, b: SdfResult) -> SdfResult {
    let d = max(a.dist, -b.dist);
    return SdfResult(d, a.color);
}

fn op_subtract_smooth(a: SdfResult, b: SdfResult, k: f32) -> SdfResult {
    let h = clamp(0.5 - 0.5 * (b.dist + a.dist) / k, 0.0, 1.0);
    let d = mix(a.dist, -b.dist, h) + k * h * (1.0 - h);
    return SdfResult(d, a.color);
}

fn op_intersect(a: SdfResult, b: SdfResult) -> SdfResult {
    if (a.dist > b.dist) { return a; }
    return b;
}

fn set_color(res: SdfResult, col: vec3<f32>) -> SdfResult {
    var out = res;
    out.color = col;
    return out;
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
        map(p + e.xyy).dist - map(p - e.xyy).dist,
        map(p + e.yxy).dist - map(p - e.yxy).dist,
        map(p + e.yyx).dist - map(p - e.yyx).dist
    ));
}

fn ray_march(ro: vec3<f32>, rd: vec3<f32>) -> SdfResult {
    var t = 0.0;
    var res = SdfResult(100.0, vec3<f32>(0.0));
    
    for (var i = 0; i < 128; i++) {
        let p = ro + rd * t;
        res = map(p);
        
        let d = abs(res.dist);
        if (d < 0.001 || t > 50.0) { 
            res.dist = t;
            break; 
        }
        t += d;
    }
    return res;
}

fn get_grid_color(p: vec3<f32>, rd: vec3<f32>) -> vec4<f32> {
    let t = -p.y / rd.y;
    if (t > 0.0 && t < 100.0) {
        let pos = p + rd * t;
        let grid = abs(fract(pos.xz - 0.5) - 0.5) / fwidth(pos.xz);
        let line = min(grid.x, grid.y);
        let color = 1.0 - min(line, 1.0);
        let alpha = color * exp(-t * 0.05) * 0.3;
        
        var col = vec3<f32>(0.5);
        if (abs(pos.x) < 0.05) { col = vec3<f32>(0.0, 0.0, 1.0); } // Z axis
        if (abs(pos.z) < 0.05) { col = vec3<f32>(1.0, 0.0, 0.0); } // X axis

        return vec4<f32>(col, alpha);
    }
    return vec4<f32>(0.0);
}

fn render_scene(uv: vec2<f32>) -> vec3<f32> {
    let ro = uniforms.cam_pos.xyz;
    let forward = normalize(uniforms.cam_front.xyz);
    let right = normalize(uniforms.cam_right.xyz);
    let up = normalize(uniforms.cam_up.xyz);
    let rd = normalize(uv.x * right + uv.y * up + 1.8 * forward);

    let res = ray_march(ro, rd);
    let t = res.dist;
    let bg_color = vec3<f32>(0.08, 0.08, 0.1);
    
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
        
        let lit_col = res.color * (diff + 0.1) + vec3<f32>(spec * 0.4) + vec3<f32>(fresnel);
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
    
    var total_color = vec3<f32>(0.0);
    
    // Global 8x8 SSAA (64 samples per pixel)
    // Grid offset goes from -0.4375 to 0.4375
    for (var iy: i32 = 0; iy < 8; iy = iy + 1) {
        for (var ix: i32 = 0; ix < 8; ix = ix + 1) {
            let offset = (vec2<f32>(f32(ix), f32(iy)) + 0.5) / 8.0 - 0.5;
            let uv = (((pixel_pos + offset - rect_min) / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, -1.0);
            total_color += render_scene(uv);
        }
    }

    return vec4<f32>(total_color / 64.0, 1.0);
}