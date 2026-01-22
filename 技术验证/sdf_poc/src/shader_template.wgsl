struct Uniforms {
    rect_data: vec4<f32>,
    time_data: vec4<f32>,
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

fn op_smooth_union(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

fn rotate_x(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(p.x, c * p.y - s * p.z, s * p.y + c * p.z);
}

fn rotate_y(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}

fn rotate_z(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
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
    for (var i = 0; i < 100; i++) {
        let p = ro + rd * t;
        let d = map(p);
        if (d < 0.001 || t > 20.0) { break; }
        t += d;
    }
    return t;
}

fn render_scene(uv: vec2<f32>) -> vec3<f32> {
    let ro = vec3<f32>(0.0, 0.0, 3.5);
    let rd = normalize(vec3<f32>(uv, -1.8));

    let t = ray_march(ro, rd);

    if (t > 20.0) {
        return vec3<f32>(0.08, 0.08, 0.1);
    }

    let p = ro + rd * t;
    let normal = calc_normal(p);
    
    let light_pos = vec3<f32>(2.0, 4.0, 3.0);
    let light_dir = normalize(light_pos - p);
    
    let diff = max(dot(normal, light_dir), 0.0);
    let view_dir = normalize(ro - p);
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    let fresnel = pow(1.0 - max(dot(normal, view_dir), 0.0), 5.0) * 0.3;
    
    let base_col = vec3<f32>(0.2, 0.55, 1.0);
    return base_col * (diff + 0.1) + vec3<f32>(spec * 0.4) + vec3<f32>(fresnel);
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    let x = f32(i32(in_vertex_index) & 1) * 4.0 - 1.0;
    let y = f32(i32(in_vertex_index) & 2) * 2.0 - 1.0;
    
    var out: VertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_pos = in.clip_position.xy;
    let rect_min = uniforms.rect_data.xy;
    let rect_size = uniforms.rect_data.zw;
    let aspect = rect_size.x / rect_size.y;

    var total_color = vec3<f32>(0.0);
    
    var local_pos = (pixel_pos + vec2<f32>(-0.25, -0.25)) - rect_min;
    total_color += render_scene(((local_pos / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, 1.0));

    local_pos = (pixel_pos + vec2<f32>(0.25, -0.25)) - rect_min;
    total_color += render_scene(((local_pos / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, 1.0));

    local_pos = (pixel_pos + vec2<f32>(-0.25, 0.25)) - rect_min;
    total_color += render_scene(((local_pos / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, 1.0));

    local_pos = (pixel_pos + vec2<f32>(0.25, 0.25)) - rect_min;
    total_color += render_scene(((local_pos / rect_size) * 2.0 - 1.0) * vec2<f32>(aspect, 1.0));

    return vec4<f32>(total_color / 4.0, 1.0);
}
