//! WGPU renderer: instanced box pipeline (world), billboard pipeline (particles/
//! shadows), instanced quad pipeline (UI + text). All geometry generated from
//! vertex_index in the shaders — no vertex buffers, only instance buffers.

pub mod camera;
pub mod text;

pub use glam::{Mat4, Vec3, Vec4};

use crate::assets::Assets;
use std::num::NonZeroU64;
use wgpu::util::DeviceExt;

pub const BOX_CAP: usize = 40000;
pub const BILLBOARD_CAP: usize = 8192;
pub const QUAD_CAP: usize = 8192;
pub const TEXT_CAP: usize = 24576;

// ----------------------------------------------------------------------
// Instance data
// ----------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoxInstance {
    pub pos: [f32; 3],
    pub quat: [f32; 4],
    pub scale: [f32; 3],
    pub uv_top: [f32; 4],
    pub uv_bottom: [f32; 4],
    pub uv_front: [f32; 4],
    pub uv_back: [f32; 4],
    pub uv_right: [f32; 4],
    pub uv_left: [f32; 4],
    pub tint: [f32; 4],
    pub emissive: f32,
    pub _pad: f32,
}

impl BoxInstance {
    pub fn new(
        pos: Vec3,
        quat: glam::Quat,
        scale: Vec3,
        uv: [[f32; 4]; 6],
        tint: Vec4,
        emissive: f32,
    ) -> Self {
        BoxInstance {
            pos: pos.to_array(),
            quat: quat.to_array(),
            scale: scale.to_array(),
            uv_top: uv[0],
            uv_bottom: uv[1],
            uv_front: uv[2],
            uv_back: uv[3],
            uv_right: uv[4],
            uv_left: uv[5],
            tint: tint.to_array(),
            emissive,
            _pad: 0.0,
        }
    }

    /// Solid unit box with one texture cell on every face (atlas rect in 0..1).
    pub fn cube(pos: Vec3, scale: Vec3, rect: [f32; 4], tint: Vec4, emissive: f32) -> Self {
        Self::new(pos, glam::Quat::IDENTITY, scale, [rect; 6], tint, emissive)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BillboardInstance {
    pub pos: [f32; 3],
    pub size: [f32; 2],
    pub uv: [f32; 4],
    pub color: [f32; 4],
    pub mode: f32, // 0 = ground quad, 1 = camera billboard
    pub _pad: f32,
}

impl BillboardInstance {
    pub fn new(pos: Vec3, size: [f32; 2], color: Vec4, mode: f32) -> Self {
        BillboardInstance {
            pos: pos.to_array(),
            size,
            uv: [0.0; 4],
            color: color.to_array(),
            mode,
            _pad: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv: [f32; 4],
    pub color: [f32; 4],
    pub flag: f32, // 0 = flat color, 1 = textured (atlas/glyph bound at draw)
    pub _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub vp: [[f32; 4]; 4],
    pub campos: [f32; 4],
    pub fogcolor: [f32; 4],
    pub fogrange: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiUniform {
    pub screen: [f32; 2],
    pub _pad: [f32; 2],
}

pub struct FrameData {
    pub boxes_atlas: Vec<BoxInstance>,
    pub boxes_skin: Vec<BoxInstance>,
    pub billboards: Vec<BillboardInstance>,
    pub quads: Vec<QuadInstance>,
    pub text: Vec<QuadInstance>,
    pub cam: CameraUniform,
}

impl Default for FrameData {
    fn default() -> Self {
        FrameData {
            boxes_atlas: Vec::with_capacity(8192),
            boxes_skin: Vec::with_capacity(256),
            billboards: Vec::with_capacity(512),
            quads: Vec::with_capacity(1024),
            text: Vec::with_capacity(2048),
            cam: CameraUniform {
                vp: Mat4::IDENTITY.to_cols_array_2d(),
                campos: [0.0; 4],
                fogcolor: [0.5, 0.7, 0.9, 1.0],
                fogrange: [30.0, 90.0, 0.0, 0.0],
            },
        }
    }
}

// ----------------------------------------------------------------------
// Shaders
// ----------------------------------------------------------------------

const BOX_SHADER: &str = r#"
struct Cam {
    vp: mat4x4<f32>,
    campos: vec4<f32>,
    fogcolor: vec4<f32>,
    fogrange: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Cam;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct BIn {
    @builtin(vertex_index) vi: u32,
    @location(0) pos: vec3<f32>,
    @location(1) quat: vec4<f32>,
    @location(2) scale: vec3<f32>,
    @location(3) uvt: vec4<f32>,
    @location(4) uvb: vec4<f32>,
    @location(5) uvf: vec4<f32>,
    @location(6) uvk: vec4<f32>,
    @location(7) uvr: vec4<f32>,
    @location(8) uvl: vec4<f32>,
    @location(9) tint: vec4<f32>,
    @location(10) emissive: f32,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) world: vec3<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) emissive: f32,
    @location(5) fi: f32,
    @location(6) suv: vec2<f32>,
};

var<private> FACE_POS: array<vec3<f32>, 36> = array<vec3<f32>, 36>(
    // top (+Y)
    vec3<f32>(-0.5, 0.5, -0.5), vec3<f32>(0.5, 0.5, -0.5), vec3<f32>(-0.5, 0.5, 0.5),
    vec3<f32>(-0.5, 0.5, 0.5), vec3<f32>(0.5, 0.5, -0.5), vec3<f32>(0.5, 0.5, 0.5),
    // bottom (-Y)
    vec3<f32>(-0.5, -0.5, 0.5), vec3<f32>(0.5, -0.5, 0.5), vec3<f32>(-0.5, -0.5, -0.5),
    vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(0.5, -0.5, 0.5), vec3<f32>(0.5, -0.5, -0.5),
    // front (-Z)
    vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(0.5, -0.5, -0.5), vec3<f32>(-0.5, 0.5, -0.5),
    vec3<f32>(-0.5, 0.5, -0.5), vec3<f32>(0.5, -0.5, -0.5), vec3<f32>(0.5, 0.5, -0.5),
    // back (+Z)
    vec3<f32>(0.5, -0.5, 0.5), vec3<f32>(-0.5, -0.5, 0.5), vec3<f32>(0.5, 0.5, 0.5),
    vec3<f32>(0.5, 0.5, 0.5), vec3<f32>(-0.5, -0.5, 0.5), vec3<f32>(-0.5, 0.5, 0.5),
    // right (+X)
    vec3<f32>(0.5, -0.5, -0.5), vec3<f32>(0.5, -0.5, 0.5), vec3<f32>(0.5, 0.5, -0.5),
    vec3<f32>(0.5, 0.5, -0.5), vec3<f32>(0.5, -0.5, 0.5), vec3<f32>(0.5, 0.5, 0.5),
    // left (-X)
    vec3<f32>(-0.5, -0.5, 0.5), vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5, 0.5, 0.5),
    vec3<f32>(-0.5, 0.5, 0.5), vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5, 0.5, -0.5)
);

var<private> FACE_NRM: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, -1.0, 0.0), vec3<f32>(0.0, 0.0, -1.0),
    vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0)
);

var<private> FACE_UVC: array<vec2<f32>, 36> = array<vec2<f32>, 36>(
    // top: MC unwrap — cell BOTTOM edge touches the front cell (front edge -> v=1),
    // and the U axis is NOT mirrored: cell-left = character-right, same as the
    // front face (the fold preserves left/right, so the top continues the front).
    vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(0.0, 0.0),
    // bottom: folded under — front edge (z=-0.5) maps to uv.y = 0
    vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 0.0),
    vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(1.0, 0.0),
    // front: v up, u flipped so the texture is NOT mirrored for the outside viewer
    vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(0.0, 0.0),
    // back (same un-mirror rule)
    vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(0.0, 0.0),
    // right (+X): u increases towards -Z (character front), un-mirrored
    vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(0.0, 0.0),
    // left (-X): u increases towards +Z (character front), un-mirrored
    vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(0.0, 0.0)
);

// MCD look: strong stylized contrast (bright sunlit tops, deep shaded sides)
var<private> FACE_SHADE: array<f32, 6> = array<f32, 6>(1.0, 0.42, 0.86, 0.72, 0.8, 0.8);

fn quat_rot(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

@vertex
fn vs_main(in: BIn) -> VOut {
    let vi = in.vi;
    let fi = vi / 6u;
    let pos_l = FACE_POS[vi] * in.scale;
    let pos_w = quat_rot(in.quat, pos_l) + in.pos;
    let nrm_w = normalize(quat_rot(in.quat, FACE_NRM[fi]));
    var uv = in.uvt;
    if (fi == 1u) { uv = in.uvb; }
    else if (fi == 2u) { uv = in.uvf; }
    else if (fi == 3u) { uv = in.uvk; }
    else if (fi == 4u) { uv = in.uvr; }
    else if (fi == 5u) { uv = in.uvl; }
    var out: VOut;
    out.clip = cam.vp * vec4<f32>(pos_w, 1.0);
    out.uv = uv.xy + FACE_UVC[vi] * uv.zw;
    out.nrm = nrm_w;
    out.world = pos_w;
    out.tint = in.tint;
    out.emissive = in.emissive;
    out.fi = f32(fi);
    let ndc = out.clip.xy / max(out.clip.w, 0.0001);
    out.suv = ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let texel = textureSample(tex, samp, in.uv);
    if (texel.a < 0.35) { discard; }
    let fi = u32(in.fi + 0.5);
    var light = max(dot(in.nrm, normalize(vec3<f32>(0.45, -0.85, 0.28))), 0.0);
    light = max(light, in.emissive);
    var lum = (0.35 + 0.65 * light) * FACE_SHADE[fi];
    lum = max(lum, in.emissive);
    var col = texel.rgb * in.tint.rgb * lum;
    // Minecraft Dungeons grade: saturation punch + warm highlights / cool shadows
    let g = clamp(dot(col, vec3<f32>(0.299, 0.587, 0.114)), 0.0, 1.0);
    col = mix(vec3<f32>(g), col, vec3<f32>(1.24));
    col *= mix(vec3<f32>(0.88, 0.94, 1.14), vec3<f32>(1.14, 1.04, 0.9), smoothstep(0.12, 0.85, g));
    let d = distance(cam.campos.xyz, in.world);
    let f = smoothstep(cam.fogrange.x, cam.fogrange.y, d);
    col = mix(col, cam.fogcolor.rgb, f);
    // cinematic vignette (darkens screen edges, MCD framing)
    let vd = distance(in.suv, vec2<f32>(0.5, 0.5));
    col *= 1.0 - smoothstep(0.42, 0.92, vd) * 0.38;
    return vec4<f32>(col, texel.a * in.tint.a);
}
"#;

const BILLBOARD_SHADER: &str = r#"
struct Cam {
    vp: mat4x4<f32>,
    campos: vec4<f32>,
    fogcolor: vec4<f32>,
    fogrange: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Cam;

struct BIn {
    @builtin(vertex_index) vi: u32,
    @location(0) pos: vec3<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec4<f32>,
    @location(3) color: vec4<f32>,
    @location(4) mode: f32,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: BIn) -> VOut {
    let ci = in.vi % 6u;
    // corner table: [(0,0),(1,0),(0,1),(0,1),(1,0),(1,1)] -> centered (-0.5..0.5)
    let cx = f32(ci == 1u || ci == 4u || ci == 5u) - 0.5;
    let cy = f32(ci == 2u || ci == 3u || ci == 5u) - 0.5;
    let c = vec2<f32>(cx, -cy);
    var off: vec3<f32>;
    if (in.mode < 0.5) {
        // ground quad
        off = vec3<f32>(c.x * in.size.x, 0.0, c.y * in.size.y);
    } else {
        // camera-facing billboard
        let to_cam = normalize(cam.campos.xyz - in.pos);
        let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), to_cam));
        let up = cross(to_cam, right);
        off = right * (c.x * in.size.x) + up * (c.y * in.size.y);
    }
    let pos_w = in.pos + off;
    var out: VOut;
    out.clip = cam.vp * vec4<f32>(pos_w, 1.0);
    var col = in.color;
    let d = distance(cam.campos.xyz, pos_w);
    let f = smoothstep(cam.fogrange.x, cam.fogrange.y, d);
    col = vec4<f32>(mix(col.rgb, cam.fogcolor.rgb, f * 0.8), col.a);
    out.color = col;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

const QUAD_SHADER: &str = r#"
struct Ui {
    screen: vec4<f32>,
};
@group(0) @binding(0) var<uniform> ui: Ui;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct BIn {
    @builtin(vertex_index) vi: u32,
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec4<f32>,
    @location(3) color: vec4<f32>,
    @location(4) flag: f32,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) flag: f32,
};

@vertex
fn vs_main(in: BIn) -> VOut {
    let ci = in.vi % 6u;
    // corner table: [(0,0),(1,0),(0,1),(0,1),(1,0),(1,1)]
    let cx = f32(ci == 1u || ci == 4u || ci == 5u);
    let cy = f32(ci == 2u || ci == 3u || ci == 5u);
    let px = in.pos + vec2<f32>(cx, cy) * in.size;
    var out: VOut;
    out.clip = vec4<f32>(
        px.x / ui.screen.x * 2.0 - 1.0,
        1.0 - px.y / ui.screen.y * 2.0,
        0.0, 1.0
    );
    out.uv = in.uv.xy + vec2<f32>(cx, cy) * in.uv.zw;
    out.color = in.color;
    out.flag = in.flag;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    if (in.flag < 0.5) {
        return in.color;
    }
    let t = textureSample(tex, samp, in.uv);
    return vec4<f32>(in.color.rgb * t.rgb, in.color.a * t.a);
}
"#;

// ----------------------------------------------------------------------
// Gfx context
// ----------------------------------------------------------------------

pub struct Gfx {
    pub window: std::sync::Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    pub size: (u32, u32),
    frames: u64,

    box_pipeline: wgpu::RenderPipeline,
    billboard_pipeline: wgpu::RenderPipeline,
    quad_pipeline: wgpu::RenderPipeline,

    cam_bg: wgpu::BindGroup,
    ui_bg: wgpu::BindGroup,
    pub atlas_bg: wgpu::BindGroup,
    pub skin_bg: wgpu::BindGroup,
    pub glyph_bg: wgpu::BindGroup,
    /// kept alive so the skin texture can be hot-swapped (hero editor)
    bgl_tex: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,

    cam_buf: wgpu::Buffer,
    ui_buf: wgpu::Buffer,
    box_atlas_buf: wgpu::Buffer,
    box_skin_buf: wgpu::Buffer,
    billboard_buf: wgpu::Buffer,
    quad_buf: wgpu::Buffer,
    text_buf: wgpu::Buffer,

    box_atlas_cap: usize,
    box_skin_cap: usize,
    billboard_cap: usize,
    quad_cap: usize,
    text_cap: usize,
}

const CAM_SIZE: u64 = 112;
const UI_SIZE: u64 = 16;

fn box_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: 160,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 12, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 28, shader_location: 2 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 40, shader_location: 3 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 56, shader_location: 4 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 72, shader_location: 5 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 88, shader_location: 6 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 104, shader_location: 7 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 120, shader_location: 8 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 136, shader_location: 9 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 152, shader_location: 10 },
        ],
    }
}

fn billboard_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<BillboardInstance>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 2 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 3 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 48, shader_location: 4 },
        ],
    }
}

fn quad_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<QuadInstance>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 2 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 3 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 48, shader_location: 4 },
        ],
    }
}

impl Gfx {
    pub async fn new(window: std::sync::Arc<winit::window::Window>, assets: &Assets, glyphs: &text::GlyphCache) -> Gfx {
        let size = window.inner_size();
        log::info!("gfx::new inner_size={size:?} scale={}", window.scale_factor());
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).expect("surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no suitable GPU adapter");
        let ainfo = adapter.get_info();
        log::info!("adapter: {} ({:?})", ainfo.name, ainfo.backend);
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                },
            )
            .await
            .expect("device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("surface config");
        config.alpha_mode = wgpu::CompositeAlphaMode::Auto;
        surface.configure(&device, &config);

        let depth_view = Self::create_depth(&device, size.width.max(1), size.height.max(1));

        // bind group layouts
        let bgl_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl-uniform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bgl_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl-tex"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // pipelines
        let shader_box = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("box-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(BOX_SHADER)),
        });
        let shader_bill = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("billboard-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(BILLBOARD_SHADER)),
        });
        let shader_quad = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(QUAD_SHADER)),
        });

        let playout_box = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("playout-box"),
            bind_group_layouts: &[&bgl_uniform, &bgl_tex],
            push_constant_ranges: &[],
        });
        let playout_bill = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("playout-bill"),
            bind_group_layouts: &[&bgl_uniform],
            push_constant_ranges: &[],
        });
        let playout_quad = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("playout-quad"),
            bind_group_layouts: &[&bgl_uniform, &bgl_tex],
            push_constant_ranges: &[],
        });

        let blend_alpha = wgpu::BlendState::ALPHA_BLENDING;
        let depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let box_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("box-pipeline"),
            layout: Some(&playout_box),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader_box,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[box_vertex_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_box,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(blend_alpha),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let billboard_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("billboard-pipeline"),
            layout: Some(&playout_bill),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader_bill,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[billboard_vertex_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_bill,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(blend_alpha),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                ..depth_stencil
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad-pipeline"),
            layout: Some(&playout_quad),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader_quad,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[quad_vertex_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_quad,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(blend_alpha),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // textures
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        if std::env::var("MD_DUMP").is_ok() {
            let _ = glyphs.image.save("/tmp/md_glyphs.png");
            let _ = assets.atlas.save("/tmp/md_atlas.png");
            log::info!("saved glyph atlas + atlas debug images");
        }
        let atlas_bg = Self::tex_bind_group(&device, &queue, &bgl_tex, &sampler, &assets.atlas);
        let skin_bg = match &assets.skin {
            Some(img) => Self::tex_bind_group(&device, &queue, &bgl_tex, &sampler, img),
            None => atlas_bg.clone(),
        };
        let glyph_bg = Self::tex_bind_group(&device, &queue, &bgl_tex, &sampler, &glyphs.image);

        // uniform buffers + bind groups
        let cam_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cam-buf"),
            contents: bytemuck::bytes_of(&CameraUniform {
                vp: Mat4::IDENTITY.to_cols_array_2d(),
                campos: [0.0; 4],
                fogcolor: [0.5, 0.7, 0.9, 1.0],
                fogrange: [30.0, 90.0, 0.0, 0.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let ui_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ui-buf"),
            contents: bytemuck::bytes_of(&UiUniform {
                screen: [size.width as f32, size.height as f32],
                _pad: [0.0; 2],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let cam_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cam-bg"),
            layout: &bgl_uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cam_buf.as_entire_binding(),
            }],
        });
        let ui_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui-bg"),
            layout: &bgl_uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ui_buf.as_entire_binding(),
            }],
        });

        let mkbuf = |label: &str, cap: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: cap as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let box_atlas_buf = mkbuf("box-atlas-buf", BOX_CAP * 160);
        let box_skin_buf = mkbuf("box-skin-buf", 1024 * 160);
        let billboard_buf = mkbuf("billboard-buf", BILLBOARD_CAP * 64);
        let quad_buf = mkbuf("quad-buf", QUAD_CAP * 64);
        let text_buf = mkbuf("text-buf", TEXT_CAP * 64);

        Gfx {
            window,
            surface,
            device,
            queue,
            config,
            depth_view,
            size: (size.width, size.height),
            box_pipeline,
            billboard_pipeline,
            quad_pipeline,
            cam_bg,
            ui_bg,
            atlas_bg,
            skin_bg,
            glyph_bg,
            bgl_tex,
            sampler,
            cam_buf,
            ui_buf,
            box_atlas_buf,
            box_skin_buf,
            billboard_buf,
            quad_buf,
            text_buf,
            box_atlas_cap: BOX_CAP * 160,
            box_skin_cap: 1024 * 160,
            frames: 0,
            billboard_cap: BILLBOARD_CAP * 64,
            quad_cap: QUAD_CAP * 64,
            text_cap: TEXT_CAP * 64,
        }
    }

    /// Hot-swap the skin texture (hero editor live preview / applied skin).
    pub fn update_skin(&mut self, img: &image::RgbaImage) {
        self.skin_bg = Self::tex_bind_group(&self.device, &self.queue, &self.bgl_tex, &self.sampler, img);
    }

    fn create_depth(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        tex.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn tex_bind_group(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        img: &image::RgbaImage,
    ) -> wgpu::BindGroup {
        let (w, h) = img.dimensions();
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tex"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(img.as_raw()),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tex-bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
            ],
        })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        let (w, h) = (w.max(1), h.max(1));
        if (w, h) == self.size {
            return;
        }
        self.size = (w, h);
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = Self::create_depth(&self.device, w, h);
    }

    fn grow<'a>(buffer: &'a mut wgpu::Buffer, cap: &mut usize, need: usize, device: &wgpu::Device) -> &'a wgpu::Buffer {
        if need > *cap {
            *cap = (need * 2).max(1024);
            *buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: *cap as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        buffer
    }

    pub fn render(&mut self, data: &mut FrameData) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                log::warn!("get_current_texture failed: {e:?} — resizing");
                self.resize(self.window.inner_size().width, self.window.inner_size().height);
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // MD_DUMP=1: re-render this frame into an offscreen texture and save to /tmp/md_dump.png
        let dump_frame: u64 = std::env::var("MD_DUMP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        let dump_now = std::env::var("MD_DUMP").is_ok() && self.frames == dump_frame;
        let mut dump_texture: Option<wgpu::Texture> = None;
        let mut dump_storage: Option<wgpu::TextureView> = None;
        let color_view_storage;
        let color_view: &wgpu::TextureView = if dump_now {
            let t = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("dump-target"),
                size: wgpu::Extent3d { width: self.config.width, height: self.config.height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let v = t.create_view(&wgpu::TextureViewDescriptor::default());
            dump_texture = Some(t);
            color_view_storage = v;
            dump_storage = Some(color_view_storage.clone());
            dump_storage.as_ref().unwrap()
        } else {
            &view
        };

        // upload uniforms
        self.queue.write_buffer(&self.cam_buf, 0, bytemuck::bytes_of(&data.cam));
        self.queue.write_buffer(
            &self.ui_buf,
            0,
            bytemuck::bytes_of(&UiUniform {
                screen: [self.size.0 as f32, self.size.1 as f32],
                _pad: [0.0; 2],
            }),
        );

        // upload instances
        if !data.boxes_atlas.is_empty() {
            let b = Self::grow(&mut self.box_atlas_buf, &mut self.box_atlas_cap, data.boxes_atlas.len() * 160, &self.device);
            self.queue.write_buffer(b, 0, bytemuck::cast_slice(&data.boxes_atlas));
        }
        if !data.boxes_skin.is_empty() {
            let b = Self::grow(&mut self.box_skin_buf, &mut self.box_skin_cap, data.boxes_skin.len() * 160, &self.device);
            self.queue.write_buffer(b, 0, bytemuck::cast_slice(&data.boxes_skin));
        }
        if !data.billboards.is_empty() {
            let b = Self::grow(&mut self.billboard_buf, &mut self.billboard_cap, data.billboards.len() * std::mem::size_of::<BillboardInstance>(), &self.device);
            self.queue.write_buffer(b, 0, bytemuck::cast_slice(&data.billboards));
        }
        if !data.quads.is_empty() {
            let b = Self::grow(&mut self.quad_buf, &mut self.quad_cap, data.quads.len() * std::mem::size_of::<QuadInstance>(), &self.device);
            self.queue.write_buffer(b, 0, bytemuck::cast_slice(&data.quads));
        }
        if !data.text.is_empty() {
            let b = Self::grow(&mut self.text_buf, &mut self.text_cap, data.text.len() * std::mem::size_of::<QuadInstance>(), &self.device);
            self.queue.write_buffer(b, 0, bytemuck::cast_slice(&data.text));
        }

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: data.cam.fogcolor[0] as f64,
                            g: data.cam.fogcolor[1] as f64,
                            b: data.cam.fogcolor[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // ---- world: boxes ----
            pass.set_pipeline(&self.box_pipeline);
            pass.set_bind_group(0, &self.cam_bg, &[]);
            if !data.boxes_atlas.is_empty() {
                pass.set_bind_group(1, &self.atlas_bg, &[]);
                pass.set_vertex_buffer(0, self.box_atlas_buf.slice(..));
                pass.draw(0..36, 0..data.boxes_atlas.len() as u32);
            }
            if !data.boxes_skin.is_empty() {
                pass.set_bind_group(1, &self.skin_bg, &[]);
                pass.set_vertex_buffer(0, self.box_skin_buf.slice(..));
                pass.draw(0..36, 0..data.boxes_skin.len() as u32);
            }
            // ---- world: billboards ----
            if !data.billboards.is_empty() {
                pass.set_pipeline(&self.billboard_pipeline);
                pass.set_bind_group(0, &self.cam_bg, &[]);
                pass.set_vertex_buffer(0, self.billboard_buf.slice(..));
                pass.draw(0..6, 0..data.billboards.len() as u32);
            }
        }

        // ---- UI pass (no depth) ----
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_bind_group(0, &self.ui_bg, &[]);
            if !data.quads.is_empty() {
                pass.set_bind_group(1, &self.atlas_bg, &[]);
                pass.set_vertex_buffer(0, self.quad_buf.slice(..));
                pass.draw(0..6, 0..data.quads.len() as u32);
            }
            if !data.text.is_empty() {
                pass.set_bind_group(1, &self.glyph_bg, &[]);
                pass.set_vertex_buffer(0, self.text_buf.slice(..));
                pass.draw(0..6, 0..data.text.len() as u32);
            }
        }

        let (tw, th) = (frame.texture.width(), frame.texture.height());
        let f = self.frames;
        self.frames += 1;
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        if let (Some(tex), Some(dt)) = (dump_texture.take(), dump_storage.take()) {
            let bytes_per_row = (tw as usize * 4).div_ceil(256) * 256;
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("dump"),
                size: (bytes_per_row * th as usize) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("dump-copy") });
            enc.copy_texture_to_buffer(
                tex.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row as u32),
                        rows_per_image: Some(th),
                    },
                },
                wgpu::Extent3d { width: tw, height: th, depth_or_array_layers: 1 },
            );
            self.queue.submit(Some(enc.finish()));
            let slice = buf.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            self.device.poll(wgpu::PollType::Wait).ok();
            {
                let data = buf.get_mapped_range(..);
                let mut img = image::RgbaImage::new(tw, th);
                for y in 0..th as usize {
                    let row = &data[y * bytes_per_row..y * bytes_per_row + tw as usize * 4];
                    for x in 0..tw as usize {
                        img.put_pixel(x as u32, y as u32, image::Rgba([row[x*4], row[x*4+1], row[x*4+2], 255]));
                    }
                }
                let _ = img.save("/tmp/md_dump.png");
                log::info!("dump written: /tmp/md_dump.png");
            }
            buf.unmap();
            let _ = dt;
        }
        if f == 0 || f % 120 == 0 {
            log::info!(
                "frame {f}: fmt={:?} size={}x{} tex={}x{} config={}x{} quads={} text={} boxes_atlas={} boxes_skin={} billboards={}",
                self.config.format,
                self.size.0,
                self.size.1,
                tw,
                th,
                self.config.width,
                self.config.height,
                data.quads.len(),
                data.text.len(),
                data.boxes_atlas.len(),
                data.boxes_skin.len(),
                data.billboards.len(),
            );
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn _unused(_: NonZeroU64) {}
}

/// Convert an atlas cell (pixels) into normalized uv rect.
pub fn rect_norm(r: crate::assets::Rect, atlas_dim: u32) -> [f32; 4] {
    [
        r[0] as f32 / atlas_dim as f32,
        r[1] as f32 / atlas_dim as f32,
        r[2] as f32 / atlas_dim as f32,
        r[3] as f32 / atlas_dim as f32,
    ]
}

pub fn cell_norm(assets: &Assets, key: &str) -> [f32; 4] {
    rect_norm(assets.cell(key), crate::assets::ATLAS_DIM)
}
