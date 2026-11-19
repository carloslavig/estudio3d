// Motor 3D na PLACA DE VIDEO (wgpu: DirectX 12 / Vulkan). Mesma cena (tres::Cena3) e mesma luz/cor do caminho em CPU,
// com MSAA 4x no lugar da superamostragem. Um quadro por vez na placa (Mutex); o resto do quadro (2D) segue em paralelo.
use crate::tres::{Cena3, Tex};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tiny_skia::Pixmap;

const L: u32 = 1080;
const A: u32 = 1920;
const LINHA: u32 = 4352; // 1080*4 alinhado a 256 bytes (exigencia da copia textura -> buffer)
const CAM_W: u32 = 1024; // camada do arranjo de texturas
const CAM_H: u32 = 256;
const CAMADAS: u32 = 64;
const NIVEIS: u32 = 11;
const SOMBRA: u32 = 4096; // mapa de sombra (lado)

const SHADER: &str = r#"
struct Mat { cor: vec4<f32>, emis: vec4<f32>, p: vec4<f32>, uvs: vec4<f32> }
@group(0) @binding(0) var<storage, read> mats: array<Mat>;
@group(0) @binding(1) var tex: texture_2d_array<f32>;
@group(0) @binding(2) var amostrador: sampler;
// luz da sombra: base (a, b, l) com o centro em .w, escala (meia largura x, meia altura y, z minimo, z maximo), liga.x
// p: posicao da luz pontual (p.w = 1) ou p.w = 0 para luz direcional; na pontual esc = (tan x, tan y, perto, longe)
struct Luz { a: vec4<f32>, b: vec4<f32>, l: vec4<f32>, esc: vec4<f32>, liga: vec4<f32>, p: vec4<f32> }
@group(0) @binding(3) var<uniform> luz: Luz;
@group(0) @binding(4) var mapa: texture_depth_2d;
@group(0) @binding(5) var compara: sampler_comparison;

fn na_luz(p: vec3<f32>) -> vec4<f32> {
    if (luz.p.w > 0.5) {
        let v = p - luz.p.xyz;
        let z = -dot(v, luz.l.xyz);
        let pe = luz.esc.z;
        let lo = luz.esc.w;
        return vec4<f32>(dot(v, luz.a.xyz) / luz.esc.x, dot(v, luz.b.xyz) / luz.esc.y, lo * (z - pe) / (lo - pe), z);
    }
    return vec4<f32>((dot(p, luz.a.xyz) - luz.a.w) / luz.esc.x, (dot(p, luz.b.xyz) - luz.b.w) / luz.esc.y, (luz.esc.w - dot(p, luz.l.xyz)) / (luz.esc.w - luz.esc.z), 1.0);
}
@vertex fn vs_sombra(i: VIn) -> @builtin(position) vec4<f32> {
    return na_luz(i.p);
}
fn h21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
fn ruido(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(h21(i), h21(i + vec2<f32>(1.0, 0.0)), u.x), mix(h21(i + vec2<f32>(0.0, 1.0)), h21(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}
fn dir_luz(p: vec3<f32>) -> vec3<f32> {
    if (luz.p.w > 0.5) {
        return normalize(luz.p.xyz - p);
    }
    return luz.l.xyz;
}
// PCF 5x5 com raio largo: sombra macia como no video
fn luz_visivel(p: vec3<f32>, n: vec3<f32>) -> f32 {
    let c = na_luz(p + n * 0.015);
    let q = c.xyz / c.w;
    let uv = vec2<f32>(q.x * 0.5 + 0.5, 0.5 - q.y * 0.5);
    let passo = luz.liga.y / 4096.0;
    var s = 0.0;
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            s += textureSampleCompareLevel(mapa, compara, uv + vec2<f32>(f32(x), f32(y)) * passo, q.z - 0.0015);
        }
    }
    return luz.liga.z + (1.0 - luz.liga.z) * s / 25.0; // sombra nunca preta: no video o chao sombreado ainda aparece
}

struct VIn { @location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>, @location(3) m: u32 }
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>, @location(3) @interpolate(flat) m: u32 }

const F: f32 = 2.7474774; // 1/tan(20 graus): fov 40
const ASP: f32 = 0.5625;
const CAM_Z: f32 = 13.0;
const PI: f32 = 3.14159265;

@vertex fn vs(i: VIn) -> VOut {
    var o: VOut;
    let zv = i.p.z - CAM_Z;
    let near = 0.1;
    let far = 100.0;
    o.pos = vec4<f32>(F / ASP * i.p.x, F * i.p.y, far / (near - far) * zv + near * far / (near - far), -zv);
    o.p = i.p;
    o.n = i.n;
    o.uv = i.uv;
    o.m = i.m;
    return o;
}

fn aces(c0: vec3<f32>) -> vec3<f32> {
    let c = c0 / 0.6;
    let i = vec3<f32>(0.59719 * c.x + 0.35458 * c.y + 0.04823 * c.z, 0.07600 * c.x + 0.90834 * c.y + 0.01566 * c.z, 0.02840 * c.x + 0.13383 * c.y + 0.83777 * c.z);
    let a = i * (i + 0.0245786) - 0.000090537;
    let b = i * (0.983729 * i + 0.4329510) + 0.238081;
    let r = a / b;
    let o = vec3<f32>(1.60475 * r.x - 0.53108 * r.y - 0.07367 * r.z, -0.10208 * r.x + 1.10813 * r.y - 0.00605 * r.z, -0.00327 * r.x - 0.07276 * r.y + 1.07602 * r.z);
    return clamp(o, vec3<f32>(0.0), vec3<f32>(1.0));
}
fn srgb(c: vec3<f32>) -> vec3<f32> {
    let baixo = c * 12.92;
    let alto = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(alto, baixo, c <= vec3<f32>(0.0031308));
}

fn luzir(n: vec3<f32>, vv: vec3<f32>, l: vec3<f32>, rad: vec3<f32>, difusa: vec3<f32>, f0: vec3<f32>, a2: f32, ndv: f32) -> vec3<f32> {
    let ndl = dot(n, l);
    if (ndl <= 0.0) { return vec3<f32>(0.0); }
    let h = normalize(l + vv);
    let ndh = max(dot(n, h), 0.0);
    let vdh = max(dot(vv, h), 0.0);
    let fres = exp2((-5.55473 * vdh - 6.98316) * vdh);
    let fr = f0 + (vec3<f32>(1.0) - f0) * fres;
    let dd = ndh * ndh * (a2 - 1.0) + 1.0;
    let d = a2 / (PI * dd * dd);
    let gv = ndl * sqrt(a2 + (1.0 - a2) * ndv * ndv);
    let gl = ndv * sqrt(a2 + (1.0 - a2) * ndl * ndl);
    let vis = 0.5 / max(gv + gl, 1e-6);
    return rad * ndl * (difusa / PI + fr * (vis * d));
}

@fragment fn fs(i: VOut) -> @location(0) vec4<f32> {
    let m = mats[i.m];
    let flags = u32(m.p.z);
    // amostra sempre (fluxo uniforme exigido pelo textureSample); so usa se o material tem textura
    let t = textureSample(tex, amostrador, vec2<f32>(i.uv.x, 1.0 - i.uv.y) * m.uvs.xy, i32(m.p.w));
    var albedo = m.cor.rgb;
    var alfa = m.cor.a;
    if ((flags & 1u) != 0u) {
        if (t.a > 0.0) { albedo = albedo * t.rgb / t.a; }
        alfa = alfa * t.a;
    }
    if ((flags & 2u) != 0u) {
        let y = (1.0 - i.uv.y) * 128.0;
        if (y - floor(y / 16.0) * 16.0 < 3.0) { albedo = albedo * 0.72; }
    }
    if ((flags & 4u) != 0u) {
        return vec4<f32>(srgb(aces(albedo)) * alfa, alfa);
    }
    var rug = m.p.x;
    if (luz.liga.x > 0.5) {
        let uv = i.uv * m.uvs.xy;
        let g = 0.5 * ruido(uv * 24.0) + 0.33 * ruido(uv * 90.0) + 0.17 * ruido(uv * 300.0);
        albedo = albedo * (0.9 + 0.2 * g);
        rug = mix(rug, 1.0, 0.3); // brilho mais espalhado, menos "plastico"
    }
    let n = normalize(i.n);
    let vv = normalize(vec3<f32>(0.0, 0.0, CAM_Z) - i.p);
    let difusa = albedo * (1.0 - m.p.y);
    let f0 = vec3<f32>(0.04) * (1.0 - m.p.y) + albedo * m.p.y;
    let r = max(rug, 0.0525);
    let a2 = (r * r) * (r * r);
    let ndv = max(abs(dot(n, vv)), 1e-4);
    var cor = difusa * (0.8 / PI) + m.emis.rgb;
    if (luz.liga.x > 0.5) {
        cor = cor + luzir(n, vv, dir_luz(i.p), vec3<f32>(luz.l.w), difusa, f0, a2, ndv) * luz_visivel(i.p, n);
    } else {
        cor = cor + luzir(n, vv, normalize(vec3<f32>(4.0, 6.0, 8.0)), vec3<f32>(1.8), difusa, f0, a2, ndv);
    }
    let dl = vec3<f32>(-5.0, -3.0, 4.0) - i.p;
    let pt = vec3<f32>(0.2541520, 0.4564110, 1.0) * 30.0; // #8ab4ff em linear
    cor = cor + luzir(n, vv, normalize(dl), pt / max(dot(dl, dl), 0.01), difusa, f0, a2, ndv);
    var final_ = aces(cor);
    if (luz.liga.w > 0.0) {
        // magno: cores mais vivas (o video tem mais saturacao que o tom do ACES)
        let lum = dot(final_, vec3<f32>(0.2126, 0.7152, 0.0722));
        final_ = clamp(mix(vec3<f32>(lum), final_, luz.liga.w), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    return vec4<f32>(srgb(final_) * alfa, alfa);
}
"#;

pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    opaco: wgpu::RenderPipeline,
    transp: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    textura: wgpu::Texture,
    tex_view: wgpu::TextureView,
    amostrador: wgpu::Sampler,
    msaa: wgpu::TextureView,
    resolvido: wgpu::Texture,
    resolvido_view: wgpu::TextureView,
    prof: wgpu::TextureView,
    leitura: wgpu::Buffer,
    sombra: wgpu::RenderPipeline,
    layout_sombra: wgpu::BindGroupLayout,
    mapa_view: wgpu::TextureView,
    compara: wgpu::Sampler,
    luz: wgpu::Buffer,
    estado: Mutex<Estado>,
}

struct Estado {
    vbuf: Option<wgpu::Buffer>,
    mbuf: Option<wgpu::Buffer>,
    /// textura (ponteiro do Arc) -> (camada, ultimo uso, Arc para nao reaproveitar o endereco)
    camadas: HashMap<usize, (u32, u64, Arc<Tex>)>,
    relogio: u64,
}

static GPU: OnceLock<Option<Gpu>> = OnceLock::new();

/// placa de video pronta para uso (ou None: segue no processador)
pub fn pronta() -> Option<&'static Gpu> {
    GPU.get_or_init(|| match pollster::block_on(Gpu::nova()) {
        Ok(g) => Some(g),
        Err(e) => {
            eprintln!("aviso: sem placa de video ({e}); 3D no processador");
            None
        }
    })
    .as_ref()
}

impl Gpu {
    async fn nova() -> Result<Gpu, String> {
        // qualquer placa, em qualquer sistema: Metal (Mac, Apple Silicon), Vulkan (Linux, Windows, Android), DX12 (Windows)
        // e OpenGL para placa antiga. Entre as que houver: dedicada, depois integrada; emulacao por software nunca
        // (o caminho do processador do proprio motor e mais rapido que ela)
        let instancia = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::all(), ..Default::default() });
        let nota = |a: &wgpu::Adapter| {
            let i = a.get_info();
            let tipo = match i.device_type {
                wgpu::DeviceType::DiscreteGpu => 30,
                wgpu::DeviceType::IntegratedGpu => 20,
                wgpu::DeviceType::Cpu => return -1,
                _ => 10,
            };
            tipo + if i.backend == wgpu::Backend::Gl { 0 } else { 5 }
        };
        let adaptador = instancia.enumerate_adapters(wgpu::Backends::all()).into_iter().filter(|a| nota(a) >= 0).max_by_key(nota).ok_or("nenhuma placa de video (so emulacao por software)")?;
        let info = adaptador.get_info();
        eprintln!("3D na placa: {} ({:?})", info.name, info.backend);
        let (device, queue) = adaptador
            .request_device(&wgpu::DeviceDescriptor { label: None, required_features: wgpu::Features::empty(), required_limits: wgpu::Limits::default(), memory_hints: wgpu::MemoryHints::Performance }, None)
            .await
            .map_err(|e| e.to_string())?;
        let modulo = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let uniforme = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2Array, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                uniforme(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison), count: None },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&layout], push_constant_ranges: &[] });
        let atributos = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Uint32];
        let pipeline = |escreve_prof: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &modulo,
                    entry_point: "vs",
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &atributos }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &modulo,
                    entry_point: "fs",
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), front_face: wgpu::FrontFace::Ccw, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: escreve_prof,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: 4, mask: !0, alpha_to_coverage_enabled: false },
                multiview: None,
                cache: None,
            })
        };
        let (opaco, transp) = (pipeline(true), pipeline(false));
        // passe da sombra: so profundidade, vista da luz; grupo proprio (o mapa nao pode ser lido e escrito no mesmo passe)
        let layout_sombra = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[uniforme(3)] });
        let pl_sombra = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&layout_sombra], push_constant_ranges: &[] });
        let sombra = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pl_sombra),
            vertex: wgpu::VertexState {
                module: &modulo,
                entry_point: "vs_sombra",
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &atributos }],
            },
            fragment: None,
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 0, slope_scale: 1.5, clamp: 0.0 },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let mapa_view = device
            .create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d { width: SOMBRA, height: SOMBRA, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let compara = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let luz = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: 96, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let alvo = |formato, amostras, uso| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d { width: L, height: A, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: amostras,
                dimension: wgpu::TextureDimension::D2,
                format: formato,
                usage: uso,
                view_formats: &[],
            })
        };
        let msaa = alvo(wgpu::TextureFormat::Rgba8Unorm, 4, wgpu::TextureUsages::RENDER_ATTACHMENT).create_view(&Default::default());
        let resolvido = alvo(wgpu::TextureFormat::Rgba8Unorm, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
        let resolvido_view = resolvido.create_view(&Default::default());
        let prof = alvo(wgpu::TextureFormat::Depth32Float, 4, wgpu::TextureUsages::RENDER_ATTACHMENT).create_view(&Default::default());
        let textura = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: CAM_W, height: CAM_H, depth_or_array_layers: CAMADAS },
            mip_level_count: NIVEIS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let tex_view = textura.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let amostrador = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });
        let leitura = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (LINHA * A) as u64, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        Ok(Gpu {
            device,
            queue,
            opaco,
            transp,
            layout,
            textura,
            tex_view,
            amostrador,
            msaa,
            resolvido,
            resolvido_view,
            prof,
            leitura,
            sombra,
            layout_sombra,
            mapa_view,
            compara,
            luz,
            estado: Mutex::new(Estado { vbuf: None, mbuf: None, camadas: HashMap::new(), relogio: 0 }),
        })
    }

    // poe a textura numa camada do arranjo (com mipmaps pre-multiplicados), reaproveitando a menos usada
    fn camada(&self, est: &mut Estado, t: &Arc<Tex>) -> Option<u32> {
        let chave = Arc::as_ptr(t) as usize;
        est.relogio += 1;
        let agora = est.relogio;
        if let Some(c) = est.camadas.get_mut(&chave) {
            c.1 = agora;
            return Some(c.0);
        }
        let (w0, h0, base) = &t.niveis[0];
        if *w0 as u32 > CAM_W || *h0 as u32 > CAM_H {
            return None;
        }
        let camada = if (est.camadas.len() as u32) < CAMADAS {
            est.camadas.len() as u32
        } else {
            let (&velha, _) = est.camadas.iter().min_by_key(|(_, v)| v.1)?;
            est.camadas.remove(&velha)?.0
        };
        // nivel 0 na camada inteira (resto transparente), depois reduz 2x2 em pre-multiplicado
        let mut atual: Vec<[f32; 4]> = vec![[0.0; 4]; (CAM_W * CAM_H) as usize];
        for y in 0..*h0 {
            for x in 0..*w0 {
                let p = base[y * w0 + x];
                atual[y * CAM_W as usize + x] = [p[0] * p[3], p[1] * p[3], p[2] * p[3], p[3]];
            }
        }
        let (mut w, mut h) = (CAM_W as usize, CAM_H as usize);
        for nivel in 0..NIVEIS {
            let bytes: Vec<u8> = atual
                .iter()
                .flat_map(|p| {
                    let e = |c: f32| (enc(c.clamp(0.0, 1.0)) * 255.0 + 0.5) as u8;
                    [e(p[0]), e(p[1]), e(p[2]), (p[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8]
                })
                .collect();
            self.queue.write_texture(
                wgpu::ImageCopyTexture { texture: &self.textura, mip_level: nivel, origin: wgpu::Origin3d { x: 0, y: 0, z: camada }, aspect: wgpu::TextureAspect::All },
                &bytes,
                wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(w as u32 * 4), rows_per_image: Some(h as u32) },
                wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
            );
            let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
            let mut prox = vec![[0.0f32; 4]; nw * nh];
            for y in 0..nh {
                for x in 0..nw {
                    let mut s = [0.0f32; 4];
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let p = atual[(y * 2 + dy).min(h - 1) * w + (x * 2 + dx).min(w - 1)];
                        for k in 0..4 {
                            s[k] += p[k] / 4.0;
                        }
                    }
                    prox[y * nw + x] = s;
                }
            }
            atual = prox;
            w = nw;
            h = nh;
        }
        est.camadas.insert(chave, (camada, agora, t.clone()));
        Some(camada)
    }

    pub fn desenhar(&self, cena: &Cena3, pm: &mut Pixmap) {
        // vertices: opacos primeiro, depois os transparentes de tras para frente
        let mut transp: Vec<(f32, usize)> = Vec::new();
        let mut verts: Vec<u32> = Vec::with_capacity(cena.tris.len() * 27);
        let por = |verts: &mut Vec<u32>, t: &crate::tres::Tri| {
            for v in &t.v {
                verts.extend_from_slice(&[v.p.x.to_bits(), v.p.y.to_bits(), v.p.z.to_bits(), v.n.x.to_bits(), v.n.y.to_bits(), v.n.z.to_bits(), v.u.to_bits(), v.v.to_bits(), t.m]);
            }
        };
        for (i, t) in cena.tris.iter().enumerate() {
            if cena.mats[t.m as usize].transp {
                transp.push((13.0 - (t.v[0].p.z + t.v[1].p.z + t.v[2].p.z) / 3.0, i));
            } else {
                por(&mut verts, t);
            }
        }
        let n_opacos = (verts.len() / 9) as u32; // em vertices
        transp.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, i) in &transp {
            por(&mut verts, &cena.tris[*i]);
        }
        let n_total = (verts.len() / 9) as u32;
        if cena.mats.is_empty() {
            return;
        }
        // so a parte da placa fica sob a trava; a mistura com o fundo acontece fora, em paralelo
        let mut copia = vec![0u8; (LINHA * A) as usize];
        {
        let mut est = self.estado.lock().unwrap_or_else(|e| e.into_inner());
        // materiais: cor+opac, emissivo, (rugosidade, metal, bandeiras, camada), escala de uv
        let mut mats: Vec<f32> = Vec::with_capacity(cena.mats.len() * 16);
        for m in &cena.mats {
            let mut flags = 0u32;
            let (mut camada, mut su, mut sv) = (0u32, 1.0f32, 1.0f32);
            if let Some(t) = &m.tex {
                if let Some(c) = self.camada(&mut est, t) {
                    flags |= 1;
                    camada = c;
                    su = t.niveis[0].0 as f32 / CAM_W as f32;
                    sv = t.niveis[0].1 as f32 / CAM_H as f32;
                }
            }
            if m.listras {
                flags |= 2;
            }
            if m.basico {
                flags |= 4;
            }
            mats.extend_from_slice(&[m.cor.x, m.cor.y, m.cor.z, m.opac, m.emis.x, m.emis.y, m.emis.z, 0.0, m.rough, m.metal, flags as f32, camada as f32, su, sv, 0.0, 0.0]);
        }
        if mats.is_empty() {
            return;
        }
        let garantir = |buf: &mut Option<wgpu::Buffer>, bytes: &[u8], uso: wgpu::BufferUsages| {
            let precisa = (bytes.len() as u64).max(64);
            if buf.as_ref().map(|b| b.size() < precisa).unwrap_or(true) {
                *buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: precisa.next_power_of_two(), usage: uso | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            }
            self.queue.write_buffer(buf.as_ref().unwrap(), 0, bytes);
        };
        garantir(&mut est.vbuf, bytemuck::cast_slice(&verts), wgpu::BufferUsages::VERTEX);
        let luz = cena.sombra.map(|l| base_da_luz(l, cena.luz_ponto, &verts[..n_opacos as usize * 9]));
        self.queue.write_buffer(&self.luz, 0, bytemuck::cast_slice(&luz.unwrap_or([0.0; 24])));
        garantir(&mut est.mbuf, bytemuck::cast_slice(&mats), wgpu::BufferUsages::STORAGE);
        let (vbuf, mbuf) = (est.vbuf.as_ref().unwrap(), est.mbuf.as_ref().unwrap());
        let grupo = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: mbuf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.tex_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.amostrador) },
                wgpu::BindGroupEntry { binding: 3, resource: self.luz.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&self.mapa_view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.compara) },
            ],
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        if luz.is_some() && n_opacos > 0 {
            let g = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout_sombra,
                entries: &[wgpu::BindGroupEntry { binding: 3, resource: self.luz.as_entire_binding() }],
            });
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.mapa_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            passe.set_pipeline(&self.sombra);
            passe.set_bind_group(0, &g, &[]);
            passe.set_vertex_buffer(0, vbuf.slice(..));
            passe.draw(0..n_opacos, 0..1);
        }
        {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa,
                    resolve_target: Some(&self.resolvido_view),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Discard },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.prof,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            passe.set_bind_group(0, &grupo, &[]);
            passe.set_vertex_buffer(0, vbuf.slice(..));
            if n_opacos > 0 {
                passe.set_pipeline(&self.opaco);
                passe.draw(0..n_opacos, 0..1);
            }
            if n_total > n_opacos {
                passe.set_pipeline(&self.transp);
                passe.draw(n_opacos..n_total, 0..1);
            }
        }
        enc.copy_texture_to_buffer(
            wgpu::ImageCopyTexture { texture: &self.resolvido, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::ImageCopyBuffer { buffer: &self.leitura, layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(LINHA), rows_per_image: Some(A) } },
            wgpu::Extent3d { width: L, height: A, depth_or_array_layers: 1 },
        );
        self.queue.submit([enc.finish()]);
        let fatia = self.leitura.slice(..);
        fatia.map_async(wgpu::MapMode::Read, |_| ());
        self.device.poll(wgpu::Maintain::Wait);
        copia.copy_from_slice(&fatia.get_mapped_range());
        self.leitura.unmap();
        }
        {
            let src = &copia;
            let dst = pm.data_mut();
            for y in 0..A as usize {
                let linha = &src[y * LINHA as usize..y * LINHA as usize + L as usize * 4];
                let d = &mut dst[y * L as usize * 4..(y + 1) * L as usize * 4];
                for (s, d) in linha.chunks_exact(4).zip(d.chunks_exact_mut(4)) {
                    let a = s[3] as u32;
                    if a == 0 {
                        continue;
                    }
                    let inv = 255 - a;
                    for k in 0..4 {
                        d[k] = (s[k] as u32 + (d[k] as u32 * inv + 127) / 255).min(255) as u8;
                    }
                }
            }
        }
    }
}

/// base ortogonal da luz e o retangulo que cobre todos os opacos, vistos da luz (o uniforme Luz do shader).
/// forca da luz = comprimento do vetor recebido (quem monta a cena calibra); maciez = raio do PCF em texels
fn base_da_luz(lv: crate::tres::V3, ponto: Option<crate::tres::V3>, verts: &[u32]) -> [f32; 24] {
    use crate::tres::{v3, V3};
    let l = lv.norm();
    let a = (if l.y.abs() < 0.9 { v3(0.0, 1.0, 0.0) } else { v3(1.0, 0.0, 0.0) }).cross(l).norm();
    let b = l.cross(a);
    let mut lim = [f32::MAX, f32::MIN, f32::MAX, f32::MIN, f32::MAX, f32::MIN];
    for v in verts.chunks_exact(9) {
        let p = V3 { x: f32::from_bits(v[0]), y: f32::from_bits(v[1]), z: f32::from_bits(v[2]) };
        for (k, e) in [p.dot(a), p.dot(b), p.dot(l)].into_iter().enumerate() {
            lim[k * 2] = lim[k * 2].min(e);
            lim[k * 2 + 1] = lim[k * 2 + 1].max(e);
        }
    }
    let (cx, cy) = ((lim[0] + lim[1]) / 2.0, (lim[2] + lim[3]) / 2.0);
    let (hx, hy) = (((lim[1] - lim[0]) / 2.0).max(0.01) * 1.02, ((lim[3] - lim[2]) / 2.0).max(0.01) * 1.02);
    let sat: f32 = std::env::var("MOTOR_SAT").ok().and_then(|v| v.parse().ok()).unwrap_or(1.35);
    let maciez: f32 = std::env::var("MOTOR_SOMBRA").ok().and_then(|v| v.parse().ok()).unwrap_or(3.5);
    if let Some(pp) = ponto {
        // luz pontual olhando para baixo (-l): perspectiva que cobre todos os opacos, perto/longe justos (mais precisao)
        let (mut tx, mut ty, mut pe, mut lo) = (0.01f32, 0.01f32, f32::MAX, 0.0f32);
        for v in verts.chunks_exact(9) {
            let d = V3 { x: f32::from_bits(v[0]), y: f32::from_bits(v[1]), z: f32::from_bits(v[2]) } - pp;
            let z = -d.dot(l);
            if z <= 0.05 {
                continue;
            }
            tx = tx.max(d.dot(a).abs() / z);
            ty = ty.max(d.dot(b).abs() / z);
            pe = pe.min(z);
            lo = lo.max(z);
        }
        let pe = (pe - 0.1).max(0.05);
        return [a.x, a.y, a.z, 0.0, b.x, b.y, b.z, 0.0, l.x, l.y, l.z, lv.len(), tx * 1.02, ty * 1.02, pe, lo + 0.1, 1.0, maciez, 0.18, sat, pp.x, pp.y, pp.z, 1.0];
    }
    [a.x, a.y, a.z, cx, b.x, b.y, b.z, cy, l.x, l.y, l.z, lv.len(), hx, hy, lim[4] - 0.1, lim[5] + 0.1, 1.0, maciez, 0.3, sat, 0.0, 0.0, 0.0, 0.0]
}

fn enc(c: f32) -> f32 {
    if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}
