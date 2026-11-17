//! The 3D viewport: renders the model with wgpu into an offscreen texture
//! that egui shows like an image. Any number of these can exist, which is
//! what lets one model appear in several live views at once.

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use glam::Mat4;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    /// The material's roughness, metallic and opacity.
    pub surface: [f32; 3],
}

/// Roughness, metallic and opacity of a surface without a material, and
/// of lines.
pub const PLAIN: [f32; 3] = [0.8, 0.0, 1.0];

/// How the 3D view draws the model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RenderMode {
    /// Surfaces in their materials' colours.
    #[default]
    Shaded,
    /// The same with every edge drawn over them.
    ShadedEdges,
    /// White surfaces with their edges: a drawing in 3D.
    HiddenLine,
    /// Edges only, the hidden ones too: what is inside and behind shows.
    Wireframe,
}

impl RenderMode {
    pub const ALL: [RenderMode; 4] = [
        RenderMode::Shaded,
        RenderMode::ShadedEdges,
        RenderMode::HiddenLine,
        RenderMode::Wireframe,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RenderMode::Shaded => "Shaded",
            RenderMode::ShadedEdges => "Shaded with edges",
            RenderMode::HiddenLine => "Hidden line",
            RenderMode::Wireframe => "Wireframe",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            RenderMode::Shaded => "sun",
            RenderMode::ShadedEdges => "render-edges",
            RenderMode::HiddenLine => "render-hidden-line",
            RenderMode::Wireframe => "render-wireframe",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            RenderMode::Shaded => "Surfaces in their materials' colours",
            RenderMode::ShadedEdges => {
                "Surfaces in colour with every edge drawn: openings and shafts read clearly"
            }
            RenderMode::HiddenLine => "White surfaces and their visible edges, like a drawing",
            RenderMode::Wireframe => {
                "Edges only, the hidden ones too: see shafts and what is inside walls and slabs"
            }
        }
    }

    /// Are the model's edges drawn?
    pub fn edges(self) -> bool {
        self != RenderMode::Shaded
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    /// The sun's view of the model, for the shadow map.
    light_view_proj: [[f32; 4]; 4],
    /// Toward the sun; w: how dark shadows are (0 none, 1 no sunlight).
    light: [f32; 4],
    eye: [f32; 4],
    /// x: one shadow map texel in metres, y: one texel in the map's uv,
    /// z: 1 with shadows.
    shadow: [f32; 4],
}

/// The sun of a frame and the shadows it casts.
#[derive(Clone, Copy, Debug)]
pub struct Lighting {
    /// Toward the sun, model axes.
    pub sun: [f32; 3],
    pub shadows: Option<Shadows>,
}

#[derive(Clone, Copy, Debug)]
pub struct Shadows {
    /// 0 none, 1 no sunlight in the shadow.
    pub strength: f32,
    /// The height shadows fall on where nothing is drawn (±0,000), if any.
    pub ground: Option<f32>,
    /// The shadow map's side, pixels (held to what the GPU can do).
    pub resolution: u32,
    /// Where the camera looks and how far it is: the shadow map covers
    /// what is around it, not the whole site, so shadows stay sharp.
    pub focus: [f32; 3],
    pub distance: f32,
}

impl Default for Lighting {
    /// The view's light as it always was, without shadows.
    fn default() -> Self {
        let l = glam::Vec3::new(0.4, -0.6, 0.85).normalize();
        Self {
            sun: l.to_array(),
            shadows: None,
        }
    }
}

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;
const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const SHADER: &str = r#"
struct U {
    view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    light: vec4<f32>,
    eye: vec4<f32>,
    shadow: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var shadow_map: texture_depth_2d;
@group(1) @binding(1) var shadow_cmp: sampler_comparison;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) surface: vec3<f32>,
};
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) world: vec3<f32>,
    @location(3) surface: vec3<f32>,
};

@vertex fn vs_main(v: VIn) -> VOut {
    var o: VOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.normal = v.normal;
    o.color = v.color;
    o.world = v.pos;
    o.surface = v.surface;
    return o;
}

// The shadow pass: depth only, from the sun.
@vertex fn vs_shadow(v: VIn) -> @builtin(position) vec4<f32> {
    return u.light_view_proj * vec4<f32>(v.pos, 1.0);
}

// How much sun reaches a point: 1 lit, 0 in full shadow. The point is
// moved a little off its surface along the normal first, so a surface
// does not shadow itself (acne); each of the 4×4 taps is a hardware
// bilinear comparison, so the edge is soft.
fn sunlight(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if (u.shadow.z < 0.5) { return 1.0; }
    let l = normalize(u.light.xyz);
    let slope = 1.0 - abs(dot(n, l));
    let p = world + n * u.shadow.x * (1.0 + 2.0 * slope);
    let c = u.light_view_proj * vec4<f32>(p, 1.0);
    let uv = vec2<f32>(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || c.z > 1.0) { return 1.0; }
    var sum = 0.0;
    for (var y = -1.5; y <= 1.5; y += 1.0) {
        for (var x = -1.5; x <= 1.5; x += 1.0) {
            sum += textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(x, y) * u.shadow.y, c.z);
        }
    }
    return mix(1.0, sum / 16.0, u.light.w);
}

// A simple studio look: the sky-and-sun lighting the view always had for
// diffuse surfaces, plus a highlight and a reflection of a soft sky that
// grow sharper as the surface gets smoother, tinted by the colour for
// metals. Colours are given in sRGB and shaded in linear space. The
// result is encoded back to sRGB: the offscreen image reaches the screen
// through egui with its values as they are, so without this every lit
// surface showed much darker than its colour.
@fragment fn fs_shaded(i: VOut) -> @location(0) vec4<f32> {
    var n = normalize(i.normal);
    let v = normalize(u.eye.xyz - i.world);
    // two-sided: light the face that is seen
    if (dot(n, v) < 0.0) { n = -n; }
    let l = normalize(u.light.xyz);
    let rough = clamp(i.surface.x, 0.04, 1.0);
    let metal = clamp(i.surface.y, 0.0, 1.0);
    let base = pow(i.color, vec3<f32>(2.2));
    var diffuse = max(dot(n, l), 0.0);
    if (diffuse > 0.0) { diffuse *= sunlight(i.world, n); }
    let sky = 0.5 + 0.5 * n.z;
    let lit = base * (1.0 - metal) * (0.30 + 0.25 * sky + 0.55 * diffuse);
    let f0 = mix(vec3<f32>(0.04), base, metal);
    let fres = pow(1.0 - max(dot(n, v), 0.0), 5.0);
    let f = f0 + (vec3<f32>(1.0) - f0) * fres * (1.0 - rough);
    let h = normalize(l + v);
    let a = rough * rough;
    let shine = clamp(2.0 / (a * a) - 2.0, 1.0, 2048.0);
    let highlight = f * pow(max(dot(n, h), 0.0), shine) * (shine + 2.0) / 8.0 * diffuse;
    let r = reflect(-v, n);
    let env = mix(vec3<f32>(0.30, 0.30, 0.32), vec3<f32>(0.95, 0.97, 1.0), 0.5 + 0.5 * r.z);
    let reflection = f * env * (1.0 - 0.6 * rough);
    let c = lit + min(highlight, vec3<f32>(4.0)) * (1.0 - 0.5 * rough) + reflection;
    return vec4<f32>(pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2)), clamp(i.surface.z, 0.05, 1.0));
}

// The ground at ±0,000, invisible but where a shadow falls on it.
@fragment fn fs_ground(i: VOut) -> @location(0) vec4<f32> {
    let shade = 1.0 - sunlight(i.world, vec3<f32>(0.0, 0.0, 1.0));
    return vec4<f32>(0.0, 0.0, 0.0, 0.55 * shade);
}

@fragment fn fs_flat(i: VOut) -> @location(0) vec4<f32> {
    return vec4<f32>(pow(i.color, vec3<f32>(2.2)), 1.0);
}
"#;

struct Mesh {
    vertices: wgpu::Buffer,
    indices: Option<wgpu::Buffer>,
    count: u32,
}

struct Targets {
    size: [u32; 2],
    msaa: wgpu::TextureView,
    resolve: wgpu::TextureView,
    depth: wgpu::TextureView,
}

pub struct Viewport3d {
    shaded: wgpu::RenderPipeline,
    /// See-through surfaces, blended over the rest after it is drawn.
    glass: wgpu::RenderPipeline,
    lines: wgpu::RenderPipeline,
    overlay_lines: wgpu::RenderPipeline,
    /// See-through surfaces drawn only where something nearer hides them.
    behind: wgpu::RenderPipeline,
    /// The selection drawn again over itself, tinted: depth equal passes,
    /// so it lies exactly on the element (MU39: a click no longer
    /// rebuilds the whole model's buffers).
    highlight: wgpu::RenderPipeline,
    /// Depth from the sun, for shadows.
    shadow_pass: wgpu::RenderPipeline,
    /// The ground at ±0,000: only the shadows on it show.
    ground: wgpu::RenderPipeline,
    ground_mesh: wgpu::Buffer,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    shadow_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    /// The shadow map, its side and the bind group that reads it.
    shadow_map: (u32, wgpu::TextureView, wgpu::BindGroup),
    /// The opaque triangles' bounds: what can cast a shadow.
    bounds: Option<(glam::Vec3, glam::Vec3)>,
    triangles: Option<Mesh>,
    glass_mesh: Option<Mesh>,
    line_mesh: Option<Mesh>,
    overlay_mesh: Option<Mesh>,
    behind_mesh: Option<Mesh>,
    sel_mesh: Option<Mesh>,
    sel_lines: Option<Mesh>,
    sel_top: Option<Mesh>,
    targets: Option<Targets>,
    texture_id: Option<egui::TextureId>,
}

impl Viewport3d {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("creabim_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("creabim_uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("creabim_bgl"),
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("creabim_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("creabim_shadow_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("creabim_shadow_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("creabim_layout"),
            bind_group_layouts: &[Some(&bgl), Some(&shadow_layout)],
            immediate_size: 0,
        });
        let depth_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("creabim_shadow_layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x3],
        };
        let shadow_pass = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("creabim_shadow_pass"),
            layout: Some(&depth_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_shadow"),
                buffers: &[Some(vertex_layout.clone())],
                compilation_options: Default::default(),
            },
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: SHADOW_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let pipeline = |topology, fragment: &str, depth_compare, blend: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fragment),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(vertex_layout.clone())],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(
                        depth_compare != wgpu::CompareFunction::Always && !blend,
                    ),
                    depth_compare: Some(depth_compare),
                    stencil: Default::default(),
                    // surfaces sit a hair behind their own edges, so edges
                    // drawn over them don't flicker
                    bias: if topology == wgpu::PrimitiveTopology::TriangleList {
                        wgpu::DepthBiasState {
                            constant: 2,
                            slope_scale: 1.5,
                            clamp: 0.0,
                        }
                    } else {
                        Default::default()
                    },
                }),
                multisample: wgpu::MultisampleState {
                    count: SAMPLES,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fragment),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend: blend.then_some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let shadow_map = shadow_target(device, &shadow_layout, &shadow_sampler, 1);
        let ground_mesh = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("creabim_ground"),
            size: (6 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            shadow_pass,
            ground: pipeline(
                wgpu::PrimitiveTopology::TriangleList,
                "fs_ground",
                wgpu::CompareFunction::Less,
                true,
            ),
            ground_mesh,
            shadow_layout,
            shadow_sampler,
            shadow_map,
            bounds: None,
            shaded: pipeline(
                wgpu::PrimitiveTopology::TriangleList,
                "fs_shaded",
                wgpu::CompareFunction::Less,
                false,
            ),
            glass: pipeline(
                wgpu::PrimitiveTopology::TriangleList,
                "fs_shaded",
                wgpu::CompareFunction::Less,
                true,
            ),
            lines: pipeline(
                wgpu::PrimitiveTopology::LineList,
                "fs_flat",
                wgpu::CompareFunction::LessEqual,
                false,
            ),
            overlay_lines: pipeline(
                wgpu::PrimitiveTopology::LineList,
                "fs_flat",
                wgpu::CompareFunction::Always,
                false,
            ),
            behind: pipeline(
                wgpu::PrimitiveTopology::TriangleList,
                "fs_shaded",
                wgpu::CompareFunction::Greater,
                true,
            ),
            highlight: pipeline(
                wgpu::PrimitiveTopology::TriangleList,
                "fs_shaded",
                wgpu::CompareFunction::LessEqual,
                true,
            ),
            uniforms,
            bind_group,
            triangles: None,
            glass_mesh: None,
            line_mesh: None,
            overlay_mesh: None,
            behind_mesh: None,
            sel_mesh: None,
            sel_lines: None,
            sel_top: None,
            targets: None,
            texture_id: None,
        }
    }

    /// Replace the geometry: shaded triangles (opaque ones, and see-through
    /// ones indexing the same vertices), depth-tested lines, and lines
    /// drawn on top of everything (2D drawings shown in 3D space).
    pub fn set_geometry(
        &mut self,
        device: &wgpu::Device,
        vertices: &[Vertex],
        indices: &[u32],
        see_through: &[u32],
        lines: &[Vertex],
        overlay: &[Vertex],
    ) {
        let buffer = |data: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: data,
                usage,
            })
        };
        // one vertex buffer for the opaque and the see-through triangles
        let shared = buffer(bytemuck::cast_slice(vertices), wgpu::BufferUsages::VERTEX);
        let tri_mesh = |indices: &[u32]| {
            (!indices.is_empty()).then(|| Mesh {
                vertices: shared.clone(),
                indices: Some(buffer(
                    bytemuck::cast_slice(indices),
                    wgpu::BufferUsages::INDEX,
                )),
                count: indices.len() as u32,
            })
        };
        self.triangles = tri_mesh(indices);
        self.bounds = bounds(vertices, indices);
        self.glass_mesh = tri_mesh(see_through);
        let line_mesh = |v: &[Vertex]| {
            (!v.is_empty()).then(|| Mesh {
                vertices: buffer(bytemuck::cast_slice(v), wgpu::BufferUsages::VERTEX),
                indices: None,
                count: v.len() as u32,
            })
        };
        self.line_mesh = line_mesh(lines);
        self.overlay_mesh = line_mesh(overlay);
    }

    /// Replace the selection's highlight: its triangles (three vertices
    /// each) tinted, its edges (depth-tested) and what shows through
    /// everything (a selected cutter's outline). Small: the selection only.
    pub fn set_selection(
        &mut self,
        device: &wgpu::Device,
        triangles: &[Vertex],
        lines: &[Vertex],
        on_top: &[Vertex],
    ) {
        let mesh = |v: &[Vertex]| {
            (!v.is_empty()).then(|| Mesh {
                vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(v),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                indices: None,
                count: v.len() as u32,
            })
        };
        self.sel_mesh = mesh(triangles);
        self.sel_lines = mesh(lines);
        self.sel_top = mesh(on_top);
    }

    /// Replace the triangles (three vertices each) drawn see-through where
    /// what is in front hides them: elements inside walls and slabs.
    pub fn set_behind(&mut self, device: &wgpu::Device, vertices: &[Vertex]) {
        self.behind_mesh = (!vertices.is_empty()).then(|| Mesh {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: None,
            count: vertices.len() as u32,
        });
    }

    fn ensure_targets(&mut self, rs: &egui_wgpu::RenderState, size: [u32; 2]) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let device = &rs.device;
        let texture = |samples, format, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let t = Targets {
            size,
            msaa: texture(
                SAMPLES,
                COLOR_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
            resolve: texture(
                1,
                COLOR_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
            depth: texture(
                SAMPLES,
                DEPTH_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
        };
        let mut renderer = rs.renderer.write();
        match self.texture_id {
            Some(id) => renderer.update_egui_texture_from_wgpu_texture(
                device,
                &t.resolve,
                wgpu::FilterMode::Linear,
                id,
            ),
            None => {
                self.texture_id = Some(renderer.register_native_texture(
                    device,
                    &t.resolve,
                    wgpu::FilterMode::Linear,
                ))
            }
        }
        self.targets = Some(t);
    }

    /// Render a frame and return the egui texture holding it.
    pub fn render(
        &mut self,
        rs: &egui_wgpu::RenderState,
        size: [u32; 2],
        view_proj: Mat4,
        eye: [f32; 3],
        background: [f64; 3],
        lighting: &Lighting,
    ) -> egui::TextureId {
        let size = [size[0].max(1), size[1].max(1)];
        self.ensure_targets(rs, size);
        let sun = glam::Vec3::from(lighting.sun).normalize_or(glam::Vec3::Z);
        // shadows only with something to cast them
        let shadows = lighting
            .shadows
            .filter(|_| self.triangles.is_some())
            .and_then(|s| Some((s, self.bounds?)));
        let mut light_view_proj = Mat4::IDENTITY;
        let mut shadow = [0.0; 4];
        let mut ground = None;
        if let Some((s, (lo, hi))) = shadows {
            let side = s
                .resolution
                .clamp(512, rs.device.limits().max_texture_dimension_2d);
            if self.shadow_map.0 != side {
                self.shadow_map =
                    shadow_target(&rs.device, &self.shadow_layout, &self.shadow_sampler, side);
            }
            let fit = fit_light(sun, lo, hi, s.focus.into(), s.distance, side);
            light_view_proj = fit.0;
            shadow = [fit.1, 1.0 / side as f32, 1.0, 0.0];
            if let Some(z) = s.ground {
                ground = Some(ground_quad(fit.2, fit.3, z));
            }
        }
        let u = Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
            light_view_proj: light_view_proj.to_cols_array_2d(),
            light: [
                sun.x,
                sun.y,
                sun.z,
                shadows.map_or(0.0, |(s, _)| s.strength.clamp(0.0, 1.0)),
            ],
            eye: [eye[0], eye[1], eye[2], 1.0],
            shadow,
        };
        rs.queue
            .write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&u));
        if let Some(g) = &ground {
            rs.queue
                .write_buffer(&self.ground_mesh, 0, bytemuck::cast_slice(g));
        }
        let t = self.targets.as_ref().expect("ensured above");
        let mut encoder = rs.device.create_command_encoder(&Default::default());
        if shadows.is_some() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("creabim_shadows"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map.1,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.shadow_pass);
            pass.set_bind_group(0, &self.bind_group, &[]);
            if let Some(m) = &self.triangles {
                pass.set_vertex_buffer(0, m.vertices.slice(..));
                if let Some(ib) = &m.indices {
                    pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..m.count, 0, 0..1);
                }
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("creabim_3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa,
                    resolve_target: Some(&t.resolve),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: background[0],
                            g: background[1],
                            b: background[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Discard,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_bind_group(1, &self.shadow_map.2, &[]);
            let ground_mesh = ground.is_some().then(|| Mesh {
                vertices: self.ground_mesh.clone(),
                indices: None,
                count: 6,
            });
            for (pipeline, mesh) in [
                (&self.shaded, &self.triangles),
                (&self.ground, &ground_mesh),
                (&self.highlight, &self.sel_mesh),
                (&self.glass, &self.glass_mesh),
                (&self.behind, &self.behind_mesh),
                (&self.lines, &self.line_mesh),
                (&self.lines, &self.sel_lines),
                (&self.overlay_lines, &self.overlay_mesh),
                (&self.overlay_lines, &self.sel_top),
            ] {
                let Some(m) = mesh else { continue };
                pass.set_pipeline(pipeline);
                pass.set_vertex_buffer(0, m.vertices.slice(..));
                match &m.indices {
                    Some(ib) => {
                        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..m.count, 0, 0..1);
                    }
                    None => pass.draw(0..m.count, 0..1),
                }
            }
        }
        rs.queue.submit([encoder.finish()]);
        self.texture_id.expect("registered in ensure_targets")
    }
}

/// A shadow map of `side` pixels and the bind group that reads it.
fn shadow_target(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    side: u32,
) -> (u32, wgpu::TextureView, wgpu::BindGroup) {
    let view = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("creabim_shadow_map"),
            size: wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("creabim_shadow_bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    (side, view, group)
}

/// The bounds of the vertices the triangles use.
fn bounds(vertices: &[Vertex], indices: &[u32]) -> Option<(glam::Vec3, glam::Vec3)> {
    let mut used = indices
        .iter()
        .filter_map(|&i| vertices.get(i as usize))
        .map(|v| glam::Vec3::from(v.pos));
    let first = used.next()?;
    Some(used.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p))))
}

/// The sun's view for the shadow map: an orthographic box along the sun
/// that holds everything able to cast a shadow, and across it only what
/// is around where the camera looks (the whole model when the camera
/// sees it all), so a building on a large site still gets sharp shadows.
/// The box moves in whole texels, so shadows don't shimmer as the camera
/// moves. Returns the matrix, one texel in metres, and the centre and
/// half side of the area covered.
fn fit_light(
    sun: glam::Vec3,
    lo: glam::Vec3,
    hi: glam::Vec3,
    focus: glam::Vec3,
    distance: f32,
    side: u32,
) -> (Mat4, f32, glam::Vec3, f32) {
    use glam::Vec3;
    let centre = (lo + hi) / 2.0;
    let radius = ((hi - lo).length() / 2.0).max(0.5);
    // what the camera sees, a generous circle around where it looks
    let near = (distance * 1.6).max(5.0);
    let (mid, half) = if near < radius {
        (focus.clamp(lo, hi), near)
    } else {
        (centre, radius)
    };
    let up = if sun.z.abs() > 0.99 { Vec3::Y } else { Vec3::Z };
    let view = Mat4::look_at_rh(centre + sun * radius * 2.0, centre, up);
    // the area covered, in the sun's view, snapped to its texels
    let texel = 2.0 * half / side as f32;
    let m = view.transform_point3(mid);
    let snap = |v: f32| (v / texel).round() * texel;
    let (x, y) = (snap(m.x), snap(m.y));
    // every corner of the model, so all its casters are in depth
    let mut zmin = f32::MAX;
    let mut zmax = f32::MIN;
    for i in 0..8 {
        let c = Vec3::new(
            if i & 1 == 0 { lo.x } else { hi.x },
            if i & 2 == 0 { lo.y } else { hi.y },
            if i & 4 == 0 { lo.z } else { hi.z },
        );
        let z = view.transform_point3(c).z;
        zmin = zmin.min(z);
        zmax = zmax.max(z);
    }
    // looking down −z: near is −zmax
    let proj = Mat4::orthographic_rh(
        x - half,
        x + half,
        y - half,
        y + half,
        -zmax - 1.0,
        -zmin + 1.0,
    );
    let covered = view.inverse().transform_point3(Vec3::new(x, y, m.z));
    (proj * view, texel, covered, half)
}

/// Two triangles of ground at height `z` around `centre`, wide enough for
/// shadows thrown long by a low sun.
fn ground_quad(centre: glam::Vec3, half: f32, z: f32) -> [Vertex; 6] {
    let r = half * 2.0;
    let v = |x: f32, y: f32| Vertex {
        pos: [centre.x + x * r, centre.y + y * r, z],
        normal: [0.0, 0.0, 1.0],
        color: [0.0; 3],
        surface: PLAIN,
    };
    [
        v(-1.0, -1.0),
        v(1.0, -1.0),
        v(1.0, 1.0),
        v(-1.0, -1.0),
        v(1.0, 1.0),
        v(-1.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn the_light_box_holds_the_model() {
        let lo = Vec3::new(-10.0, -5.0, 0.0);
        let hi = Vec3::new(10.0, 5.0, 8.0);
        let sun = Vec3::new(0.3, -0.5, 0.8).normalize();
        let (m, texel, _, _) = fit_light(sun, lo, hi, Vec3::ZERO, 100.0, 4096);
        assert!(texel > 0.0 && texel < 0.02);
        for i in 0..8 {
            let c = Vec3::new(
                if i & 1 == 0 { lo.x } else { hi.x },
                if i & 2 == 0 { lo.y } else { hi.y },
                if i & 4 == 0 { lo.z } else { hi.z },
            );
            let p = m.project_point3(c);
            assert!(p.x.abs() <= 1.0 && p.y.abs() <= 1.0, "{c} → {p}");
            assert!((0.0..=1.0).contains(&p.z), "{c} → {p}");
        }
    }

    #[test]
    fn a_close_camera_gets_sharper_shadows() {
        let lo = Vec3::new(-500.0, -500.0, 0.0);
        let hi = Vec3::new(500.0, 500.0, 20.0);
        let sun = Vec3::new(0.3, -0.5, 0.8).normalize();
        let far = fit_light(sun, lo, hi, Vec3::ZERO, 2000.0, 4096).1;
        let near = fit_light(sun, lo, hi, Vec3::ZERO, 20.0, 4096).1;
        assert!(near * 10.0 < far, "{near} {far}");
        // casters far away still fall within depth
        let (m, ..) = fit_light(sun, lo, hi, Vec3::ZERO, 20.0, 4096);
        let p = m.project_point3(Vec3::new(500.0, 500.0, 20.0));
        assert!((0.0..=1.0).contains(&p.z));
    }

    #[test]
    fn the_shader_compiles() {
        let module = naga::front::wgsl::parse_str(SHADER).expect("WGSL parses");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("WGSL validates");
    }
}
