// Motor 3D em CPU (rasterizador proprio), feito para reproduzir o palco do Remotion (src/base/Palco.tsx):
// camera em (0,0,13) com fov 40, luz ambiente 0,8, direcional (4,6,8) 1,8, pontual (-5,-3,4) 30 azulada,
// material PBR do three.js (MeshStandardMaterial sem mapa de ambiente), tone mapping ACES e saida sRGB (padroes do react-three-fiber).
// Desenha em faixas (pouca memoria), com superamostragem (ss x ss amostras por pixel) para as bordas ficarem lisas.
use std::f32::consts::PI;
use std::ops::{Add, Mul, Neg, Sub};
use std::sync::Arc;
use tiny_skia::Pixmap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}
impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, s: f32) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl V3 {
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 0.0 { self * (1.0 / l) } else { self }
    }
    pub fn vezes(self, o: V3) -> V3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
}

/// matriz 4x4 (linhas), aplicada a vetores coluna, como o three.js
#[derive(Clone, Copy, Debug)]
pub struct M4(pub [[f32; 4]; 4]);
impl M4 {
    pub fn id() -> M4 {
        M4([[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]])
    }
    pub fn mul(&self, o: &M4) -> M4 {
        let mut r = [[0.0f32; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                r[i][j] = (0..4).map(|k| self.0[i][k] * o.0[k][j]).sum();
            }
        }
        M4(r)
    }
    pub fn trans(p: V3) -> M4 {
        let mut m = M4::id();
        m.0[0][3] = p.x;
        m.0[1][3] = p.y;
        m.0[2][3] = p.z;
        m
    }
    pub fn escala(s: f32) -> M4 {
        let mut m = M4::id();
        m.0[0][0] = s;
        m.0[1][1] = s;
        m.0[2][2] = s;
        m
    }
    pub fn rx(a: f32) -> M4 {
        let (s, c) = a.sin_cos();
        M4([[1., 0., 0., 0.], [0., c, -s, 0.], [0., s, c, 0.], [0., 0., 0., 1.]])
    }
    pub fn ry(a: f32) -> M4 {
        let (s, c) = a.sin_cos();
        M4([[c, 0., s, 0.], [0., 1., 0., 0.], [-s, 0., c, 0.], [0., 0., 0., 1.]])
    }
    pub fn rz(a: f32) -> M4 {
        let (s, c) = a.sin_cos();
        M4([[c, -s, 0., 0.], [s, c, 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]])
    }
    /// <group position rotation(XYZ) scale>: T * Rx * Ry * Rz * S
    pub fn trs(pos: V3, rot: V3, s: f32) -> M4 {
        M4::trans(pos).mul(&M4::rx(rot.x)).mul(&M4::ry(rot.y)).mul(&M4::rz(rot.z)).mul(&M4::escala(s))
    }
    /// filho: self * local
    pub fn e(&self, pos: V3, rot: V3, s: f32) -> M4 {
        self.mul(&M4::trs(pos, rot, s))
    }
    pub fn p(&self, v: V3) -> V3 {
        let m = &self.0;
        v3(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z + m[0][3],
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z + m[1][3],
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z + m[2][3],
        )
    }
    pub fn d(&self, v: V3) -> V3 {
        let m = &self.0;
        v3(m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z, m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z, m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z)
    }
}

pub fn srgb_linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}
fn linear_srgb(c: f32) -> f32 {
    if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}
/// "#rrggbb" -> cor linear (o three.js converte as cores dos materiais de sRGB para linear)
pub fn hex(s: &str) -> V3 {
    let s = s.trim_start_matches('#');
    let c = |i: usize| srgb_linear(u8::from_str_radix(s.get(i..i + 2).unwrap_or("ff"), 16).unwrap_or(255) as f32 / 255.0);
    v3(c(0), c(2), c(4))
}

/// textura com mipmaps (cores lineares, alfa reto); v=1 e o topo da imagem (flipY do three.js)
pub struct Tex {
    pub(crate) niveis: Vec<(usize, usize, Vec<[f32; 4]>)>,
}
impl Tex {
    /// a partir de um Pixmap do tiny-skia (pre-multiplicado, sRGB)
    pub fn de_pixmap(pm: &Pixmap) -> Tex {
        let (w, h) = (pm.width() as usize, pm.height() as usize);
        let base: Vec<[f32; 4]> = pm
            .data()
            .chunks_exact(4)
            .map(|p| {
                let a = p[3] as f32 / 255.0;
                if a <= 0.0 {
                    return [0.0; 4];
                }
                [srgb_linear(p[0] as f32 / 255.0 / a), srgb_linear(p[1] as f32 / 255.0 / a), srgb_linear(p[2] as f32 / 255.0 / a), a]
            })
            .collect();
        let mut niveis = vec![(w, h, base)];
        while {
            let (w, h, _) = niveis.last().unwrap();
            *w > 1 && *h > 1
        } {
            let (w, h, d) = niveis.last().unwrap();
            let (nw, nh) = (w / 2, h / 2);
            let mut n = vec![[0.0f32; 4]; nw * nh];
            for y in 0..nh {
                for x in 0..nw {
                    let mut s = [0.0f32; 4];
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let p = d[(y * 2 + dy) * w + x * 2 + dx];
                        // media pre-multiplicada para a borda transparente nao escurecer
                        for k in 0..3 {
                            s[k] += p[k] * p[3];
                        }
                        s[3] += p[3];
                    }
                    n[y * nw + x] = if s[3] > 0.0 { [s[0] / s[3], s[1] / s[3], s[2] / s[3], s[3] / 4.0] } else { [0.0; 4] };
                }
            }
            niveis.push((nw, nh, n));
        }
        Tex { niveis }
    }
    fn texel_area(&self) -> f32 {
        (self.niveis[0].0 * self.niveis[0].1) as f32
    }
    fn amostra(&self, u: f32, v: f32, nivel: usize) -> [f32; 4] {
        let (w, h, d) = &self.niveis[nivel.min(self.niveis.len() - 1)];
        let (w, h) = (*w, *h);
        let x = (u.clamp(0.0, 1.0) * w as f32 - 0.5).clamp(0.0, (w - 1) as f32);
        let y = ((1.0 - v.clamp(0.0, 1.0)) * h as f32 - 0.5).clamp(0.0, (h - 1) as f32);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let p = |xx: usize, yy: usize| d[yy * w + xx];
        let mut r = [0.0f32; 4];
        let pesos = [((1.0 - fx) * (1.0 - fy), p(x0, y0)), (fx * (1.0 - fy), p(x1, y0)), ((1.0 - fx) * fy, p(x0, y1)), (fx * fy, p(x1, y1))];
        let mut a = 0.0;
        for (wgt, c) in pesos {
            for k in 0..3 {
                r[k] += c[k] * c[3] * wgt;
            }
            a += c[3] * wgt;
        }
        if a > 0.0 {
            [r[0] / a, r[1] / a, r[2] / a, a]
        } else {
            [0.0; 4]
        }
    }
}

#[derive(Clone)]
pub struct Mat {
    pub cor: V3,
    pub rough: f32,
    pub metal: f32,
    pub emis: V3,
    pub opac: f32,
    pub transp: bool,
    /// MeshBasicMaterial: sem luz
    pub basico: bool,
    pub tex: Option<Arc<Tex>>,
    /// serrilha das moedas (textura de listras do Maquete.tsx), multiplicada pela cor
    pub listras: bool,
}
impl Mat {
    pub fn std(cor: &str) -> Mat {
        Mat { cor: hex(cor), rough: 1.0, metal: 0.0, emis: V3::default(), opac: 1.0, transp: false, basico: false, tex: None, listras: false }
    }
    pub fn r(mut self, r: f32) -> Mat {
        self.rough = r;
        self
    }
    pub fn m(mut self, m: f32) -> Mat {
        self.metal = m;
        self
    }
    pub fn e(mut self, cor: &str, k: f32) -> Mat {
        self.emis = hex(cor) * k;
        self
    }
    pub fn o(mut self, o: f32) -> Mat {
        self.opac = o;
        self.transp = true;
        self
    }
    pub fn basico(tex: Arc<Tex>) -> Mat {
        Mat { cor: v3(1.0, 1.0, 1.0), rough: 1.0, metal: 0.0, emis: V3::default(), opac: 1.0, transp: true, basico: true, tex: Some(tex), listras: false }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Vert {
    pub(crate) p: V3,
    pub(crate) n: V3,
    pub(crate) u: f32,
    pub(crate) v: f32,
}
pub(crate) struct Tri {
    pub(crate) v: [Vert; 3],
    pub(crate) m: u32,
}

#[derive(Default)]
pub struct Cena3 {
    pub(crate) tris: Vec<Tri>,
    pub(crate) mats: Vec<Mat>,
    /// luz que projeta sombra (direcao para a luz, no espaco da camera); so o visual magno liga, e so na placa de video
    pub sombra: Option<V3>,
    /// se houver: a luz e um ponto (no espaco da camera) e `sombra` passa a ser o "para cima" da cena (x forca);
    /// as sombras saem para fora em volta desse ponto (magno: lampada acima do cofre)
    pub luz_ponto: Option<V3>,
}

impl Cena3 {
    pub fn mat(&mut self, m: Mat) -> u32 {
        self.mats.push(m);
        (self.mats.len() - 1) as u32
    }
    // triangulo com a frente virada para o lado das normais (o three.js usa anti-horario)
    fn tri(&mut self, m: &M4, a: (V3, V3, f32, f32), b: (V3, V3, f32, f32), c: (V3, V3, f32, f32), mat: u32) {
        let mk = |q: (V3, V3, f32, f32)| Vert { p: m.p(q.0), n: m.d(q.1).norm(), u: q.2, v: q.3 };
        let (va, mut vb, mut vc) = (mk(a), mk(b), mk(c));
        let face = (vb.p - va.p).cross(vc.p - va.p);
        if face.dot(va.n + vb.n + vc.n) < 0.0 {
            std::mem::swap(&mut vb, &mut vc);
        }
        self.tris.push(Tri { v: [va, vb, vc], m: mat });
    }
    pub fn caixa(&mut self, m: &M4, w: f32, h: f32, d: f32, mat: u32) {
        let (x, y, z) = (w / 2.0, h / 2.0, d / 2.0);
        // (normal, eixo u, eixo v)
        for (n, a, b) in [
            (v3(1., 0., 0.), v3(0., 0., -z), v3(0., y, 0.)),
            (v3(-1., 0., 0.), v3(0., 0., z), v3(0., y, 0.)),
            (v3(0., 1., 0.), v3(x, 0., 0.), v3(0., 0., -z)),
            (v3(0., -1., 0.), v3(x, 0., 0.), v3(0., 0., z)),
            (v3(0., 0., 1.), v3(x, 0., 0.), v3(0., y, 0.)),
            (v3(0., 0., -1.), v3(-x, 0., 0.), v3(0., y, 0.)),
        ] {
            let c = v3(n.x * x, n.y * y, n.z * z);
            let p = |su: f32, sv: f32| (c + a * su + b * sv, n, (su + 1.0) / 2.0, (sv + 1.0) / 2.0);
            self.tri(m, p(-1., -1.), p(1., -1.), p(1., 1.), mat);
            self.tri(m, p(-1., -1.), p(1., 1.), p(-1., 1.), mat);
        }
    }
    /// CylinderGeometry(raioTopo, raioBase, altura, segmentos) do three.js, com tampas e as mesmas normais/uv
    /// caixa com quinas arredondadas (raio r): 3 caixas + 12 cilindros nas arestas + 8 esferas nos cantos
    pub fn caixa_r(&mut self, m: &M4, w: f32, h: f32, d: f32, r: f32, mat: u32) {
        let r = r.min(w / 2.0).min(h / 2.0).min(d / 2.0);
        if r <= 0.002 {
            return self.caixa(m, w, h, d, mat);
        }
        let (x, y, z) = (w / 2.0 - r, h / 2.0 - r, d / 2.0 - r);
        let o = V3::default();
        self.caixa(m, w, h - 2.0 * r, d - 2.0 * r, mat);
        self.caixa(m, w - 2.0 * r, h, d - 2.0 * r, mat);
        self.caixa(m, w - 2.0 * r, h - 2.0 * r, d, mat);
        for sx in [-1.0f32, 1.0] {
            for sz in [-1.0f32, 1.0] {
                self.cilindro(&m.e(v3(sx * x, 0.0, sz * z), o, 1.0), r, r, 2.0 * y, 12, mat);
                self.cilindro(&m.e(v3(0.0, sx * y, sz * z), v3(0.0, 0.0, PI / 2.0), 1.0), r, r, 2.0 * x, 12, mat);
                self.cilindro(&m.e(v3(sx * x, sz * y, 0.0), v3(PI / 2.0, 0.0, 0.0), 1.0), r, r, 2.0 * z, 12, mat);
                for sy in [-1.0f32, 1.0] {
                    self.esfera(&m.e(v3(sx * x, sy * y, sz * z), o, 1.0), r, 12, mat);
                }
            }
        }
    }

    pub fn cilindro(&mut self, m: &M4, rt: f32, rb: f32, h: f32, seg: usize, mat: u32) {
        let slope = (rb - rt) / h;
        let anel = |k: usize, r: f32, y: f32| {
            let th = k as f32 / seg as f32 * 2.0 * PI;
            (v3(r * th.sin(), y, r * th.cos()), v3(th.sin(), slope, th.cos()).norm())
        };
        for k in 0..seg {
            let u0 = k as f32 / seg as f32;
            let u1 = (k + 1) as f32 / seg as f32;
            let (a, na) = anel(k, rt, h / 2.0);
            let (b, nb) = anel(k, rb, -h / 2.0);
            let (c, nc) = anel(k + 1, rb, -h / 2.0);
            let (d, nd) = anel(k + 1, rt, h / 2.0);
            if rt > 0.0 {
                self.tri(m, (a, na, u0, 1.0), (b, nb, u0, 0.0), (d, nd, u1, 1.0), mat);
            }
            if rb > 0.0 {
                self.tri(m, (b, nb, u0, 0.0), (c, nc, u1, 0.0), (d, nd, u1, 1.0), mat);
            }
        }
        for (r, y, sinal) in [(rt, h / 2.0, 1.0f32), (rb, -h / 2.0, -1.0)] {
            if r <= 0.0 {
                continue;
            }
            let n = v3(0.0, sinal, 0.0);
            for k in 0..seg {
                let p = |kk: usize| {
                    let th = kk as f32 / seg as f32 * 2.0 * PI;
                    (v3(r * th.sin(), y, r * th.cos()), n, th.cos() * 0.5 + 0.5, th.sin() * 0.5 * sinal + 0.5)
                };
                self.tri(m, p(k), p(k + 1), (v3(0.0, y, 0.0), n, 0.5, 0.5), mat);
            }
        }
    }
    /// esfera UV (copas das arvores do visual magno)
    pub fn esfera(&mut self, m: &M4, r: f32, seg: usize, mat: u32) {
        let aneis = (seg / 2).max(3);
        let ponto = |i: usize, j: usize| {
            let (th, ph) = (j as f32 / seg as f32 * 2.0 * PI, i as f32 / aneis as f32 * PI);
            let n = v3(ph.sin() * th.sin(), ph.cos(), ph.sin() * th.cos());
            (n * r, n, j as f32 / seg as f32, 1.0 - i as f32 / aneis as f32)
        };
        for i in 0..aneis {
            for j in 0..seg {
                let (a, b, cc, d) = (ponto(i, j), ponto(i + 1, j), ponto(i + 1, j + 1), ponto(i, j + 1));
                if i > 0 {
                    self.tri(m, a, b, d, mat);
                }
                if i + 1 < aneis {
                    self.tri(m, b, cc, d, mat);
                }
            }
        }
    }
    /// telhado de duas aguas: prisma triangular ao longo de x (largura w, profundidade d, altura h), base em y=0
    pub fn prisma(&mut self, m: &M4, w: f32, h: f32, d: f32, mat: u32) {
        let (x, z) = (w / 2.0, d / 2.0);
        let topo = |sx: f32| v3(sx, h, 0.0);
        for s in [1.0f32, -1.0] {
            let n = v3(0.0, z, s * h).norm();
            let (a, b) = (v3(-x, 0.0, s * z), v3(x, 0.0, s * z));
            self.tri(m, (a, n, 0.0, 0.0), (b, n, 1.0, 0.0), (topo(x), n, 1.0, 1.0), mat);
            self.tri(m, (a, n, 0.0, 0.0), (topo(x), n, 1.0, 1.0), (topo(-x), n, 0.0, 1.0), mat);
            let ne = v3(s, 0.0, 0.0);
            self.tri(m, (v3(s * x, 0.0, z), ne, 0.0, 0.0), (v3(s * x, 0.0, -z), ne, 1.0, 0.0), (topo(s * x), ne, 0.5, 1.0), mat);
        }
    }
    /// PlaneGeometry(w, h): no plano XY, virado para +z
    pub fn plano(&mut self, m: &M4, w: f32, h: f32, mat: u32) {
        let n = v3(0., 0., 1.);
        let p = |x: f32, y: f32| (v3(x * w / 2.0, y * h / 2.0, 0.0), n, (x + 1.0) / 2.0, (y + 1.0) / 2.0);
        self.tri(m, p(-1., 1.), p(-1., -1.), p(1., -1.), mat);
        self.tri(m, p(-1., 1.), p(1., -1.), p(1., 1.), mat);
    }
}

const CAM_Z: f32 = 13.0;
const NADA: u32 = u32::MAX;

struct Prep {
    sx: [f32; 3],
    sy: [f32; 3],
    iz: [f32; 3],
    area: f32,
    y0: i32,
    y1: i32,
    x0: i32,
    x1: i32,
    nivel: usize,
    prof: f32,
}

// ACES do three.js (ACESFilmicToneMapping, exposicao 1)
fn aces(c: V3) -> V3 {
    let c = c * (1.0 / 0.6);
    let i = v3(0.59719 * c.x + 0.35458 * c.y + 0.04823 * c.z, 0.07600 * c.x + 0.90834 * c.y + 0.01566 * c.z, 0.02840 * c.x + 0.13383 * c.y + 0.83777 * c.z);
    let f = |v: f32| (v * (v + 0.0245786) - 0.000090537) / (v * (0.983729 * v + 0.4329510) + 0.238081);
    let r = v3(f(i.x), f(i.y), f(i.z));
    let o = v3(
        1.60475 * r.x - 0.53108 * r.y - 0.07367 * r.z,
        -0.10208 * r.x + 1.10813 * r.y - 0.00605 * r.z,
        -0.00327 * r.x - 0.07276 * r.y + 1.07602 * r.z,
    );
    v3(o.x.clamp(0.0, 1.0), o.y.clamp(0.0, 1.0), o.z.clamp(0.0, 1.0))
}

struct Luz {
    dir_l: V3,
    dir_cor: V3,
    pt_pos: V3,
    pt_cor: V3,
    amb: f32,
}

fn sombrear(mat: &Mat, p: V3, n: V3, u: f32, v: f32, nivel: usize, luz: &Luz) -> (V3, f32) {
    let mut albedo = mat.cor;
    let mut alfa = mat.opac;
    if let Some(t) = &mat.tex {
        let c = t.amostra(u, v, nivel);
        albedo = albedo.vezes(v3(c[0], c[1], c[2]));
        alfa *= c[3];
    }
    if mat.listras && ((1.0 - v) * 128.0).rem_euclid(16.0) < 3.0 {
        albedo = albedo * 0.72; // rgba(0,0,0,0.28) sobre a cor
    }
    if mat.basico {
        return (aces_srgb(albedo), alfa);
    }
    static SEM: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *SEM.get_or_init(|| std::env::var("MOTOR_SEM_LUZ").is_ok()) {
        return (albedo, alfa);
    }
    let vv = (v3(0.0, 0.0, CAM_Z) - p).norm();
    let difusa = albedo * (1.0 - mat.metal);
    let f0 = v3(0.04, 0.04, 0.04) * (1.0 - mat.metal) + albedo * mat.metal;
    let rough = mat.rough.max(0.0525);
    let a2 = (rough * rough) * (rough * rough);
    let ndv = n.dot(vv).abs().max(1e-4);
    let mut cor = difusa * (luz.amb / PI) + mat.emis;
    let mut luzir = |l: V3, rad: V3| {
        let ndl = n.dot(l);
        if ndl <= 0.0 {
            return;
        }
        let h = (l + vv).norm();
        let ndh = n.dot(h).max(0.0);
        let vdh = vv.dot(h).max(0.0);
        let fres = (-5.55473 * vdh - 6.98316) * vdh;
        let fr = f0 + (v3(1.0, 1.0, 1.0) - f0) * fres.exp2();
        let dd = ndh * ndh * (a2 - 1.0) + 1.0;
        let d = a2 / (PI * dd * dd);
        let gv = ndl * (a2 + (1.0 - a2) * ndv * ndv).sqrt();
        let gl = ndv * (a2 + (1.0 - a2) * ndl * ndl).sqrt();
        let vis = 0.5 / (gv + gl).max(1e-6);
        let irr = rad * ndl;
        cor = cor + irr.vezes(difusa * (1.0 / PI) + fr * (vis * d));
    };
    luzir(luz.dir_l, luz.dir_cor);
    let dl = luz.pt_pos - p;
    let dist2 = dl.dot(dl);
    luzir(dl.norm(), luz.pt_cor * (1.0 / dist2.max(0.01)));
    (aces_srgb(cor), alfa)
}

fn aces_srgb(c: V3) -> V3 {
    static TAB: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
    let tab = TAB.get_or_init(|| (0..=4096).map(|i| linear_srgb(i as f32 / 4096.0)).collect());
    let t = aces(c);
    let e = |x: f32| tab[(x * 4096.0) as usize];
    v3(e(t.x), e(t.y), e(t.z))
}

/// desenha a cena por cima de `pm` (1080x1920), como o canvas transparente do ThreeCanvas por cima do fundo
pub fn desenhar(cena: &Cena3, pm: &mut Pixmap, ss: usize) {
    // placa de video primeiro (ESTUDIO_3D=cpu forca o caminho do processador)
    if std::env::var("ESTUDIO_3D").map(|v| v != "cpu").unwrap_or(true) {
        if let Some(g) = crate::gpu::pronta() {
            return g.desenhar(cena, pm);
        }
    }
    desenhar_cpu(cena, pm, ss)
}

fn desenhar_cpu(cena: &Cena3, pm: &mut Pixmap, ss: usize) {
    let _t0 = std::time::Instant::now();
    let (pw, ph) = (pm.width() as usize, pm.height() as usize);
    let (w, h) = (pw * ss, ph * ss);
    let f = 1.0 / (20.0f32.to_radians()).tan();
    let aspecto = pw as f32 / ph as f32;
    let luz = Luz {
        dir_l: v3(4.0, 6.0, 8.0).norm(),
        dir_cor: v3(1.8, 1.8, 1.8),
        pt_pos: v3(-5.0, -3.0, 4.0),
        pt_cor: hex("#8ab4ff") * 30.0,
        amb: 0.8,
    };

    let mut opacos: Vec<(usize, Prep)> = Vec::new();
    let mut transp: Vec<(usize, Prep)> = Vec::new();
    for (i, t) in cena.tris.iter().enumerate() {
        let mut sx = [0.0f32; 3];
        let mut sy = [0.0f32; 3];
        let mut iz = [0.0f32; 3];
        let mut ok = true;
        for k in 0..3 {
            let p = t.v[k].p;
            let z = CAM_Z - p.z; // distancia a frente da camera
            if z < 0.1 {
                ok = false;
                break;
            }
            sx[k] = (f / aspecto * p.x / z + 1.0) * 0.5 * w as f32;
            sy[k] = (1.0 - f * p.y / z) * 0.5 * h as f32;
            iz[k] = 1.0 / z;
        }
        if !ok {
            continue;
        }
        let area = (sx[1] - sx[0]) * (sy[2] - sy[0]) - (sx[2] - sx[0]) * (sy[1] - sy[0]);
        if area >= 0.0 {
            continue; // de costas (FrontSide) ou degenerado
        }
        let mat = &cena.mats[t.m as usize];
        // nivel do mipmap pela razao texels/amostras do triangulo
        let nivel = match &mat.tex {
            Some(tx) => {
                let uva = ((t.v[1].u - t.v[0].u) * (t.v[2].v - t.v[0].v) - (t.v[2].u - t.v[0].u) * (t.v[1].v - t.v[0].v)).abs() * tx.texel_area();
                (0.5 * (uva / area.abs()).max(1.0).log2()) as usize
            }
            None => 0,
        };
        let pr = Prep {
            y0: sy.iter().cloned().fold(f32::MAX, f32::min).floor().max(0.0) as i32,
            y1: (sy.iter().cloned().fold(f32::MIN, f32::max).ceil() as i32).min(h as i32 - 1),
            x0: sx.iter().cloned().fold(f32::MAX, f32::min).floor().max(0.0) as i32,
            x1: (sx.iter().cloned().fold(f32::MIN, f32::max).ceil() as i32).min(w as i32 - 1),
            prof: 3.0 / (iz[0] + iz[1] + iz[2]),
            sx,
            sy,
            iz,
            area,
            nivel,
        };
        if pr.x0 > pr.x1 || pr.y0 > pr.y1 {
            continue;
        }
        if mat.transp { transp.push((i, pr)) } else { opacos.push((i, pr)) }
    }
    let mut nivel_de = vec![0usize; cena.tris.len()];
    for (i, p) in &opacos {
        nivel_de[*i] = p.nivel;
    }
    transp.sort_by(|a, b| b.1.prof.partial_cmp(&a.1.prof).unwrap_or(std::cmp::Ordering::Equal));

    let faixa = 64 * ss;
    let mut prof = vec![f32::MAX; w * faixa];
    let mut id = vec![NADA; w * faixa];
    let mut bar = vec![(0.0f32, 0.0f32); w * faixa];
    let mut cor = vec![[0.0f32; 4]; w * faixa];
    let dados = pm.data_mut();

    let attr = |t: &Tri, b1: f32, b2: f32| {
        let b0 = 1.0 - b1 - b2;
        let [a, b, c] = t.v;
        (a.p * b0 + b.p * b1 + c.p * b2, (a.n * b0 + b.n * b1 + c.n * b2).norm(), a.u * b0 + b.u * b1 + c.u * b2, a.v * b0 + b.v * b1 + c.v * b2)
    };

    let mut tp = [_t0.elapsed().as_secs_f64(), 0.0, 0.0, 0.0, 0.0];
    let mut y_ini = 0;
    while y_ini < h {
        let y_fim = (y_ini + faixa).min(h);
        let linhas = y_fim - y_ini;
        prof[..w * linhas].fill(f32::MAX);
        id[..w * linhas].fill(NADA);
        cor[..w * linhas].fill([0.0; 4]);
        // varre um triangulo dentro da faixa, chamando `f(indice, profundidade, b1, b2)` em cada amostra coberta
        let _t = std::time::Instant::now();
        for (i, pr) in &opacos {
            varrer(pr, y_ini, y_fim, w, |k, z, b1, b2| {
                if z < prof[k] {
                    prof[k] = z;
                    id[k] = *i as u32;
                    bar[k] = (b1, b2);
                }
            });
        }

        tp[1] += _t.elapsed().as_secs_f64(); let _t = std::time::Instant::now();
        // 1) por pixel: se as ss x ss amostras sao do mesmo triangulo, guarda o triangulo e o baricentro medio
        let lo = linhas / ss;
        let mut uni = vec![NADA; pw * lo];
        let mut ubar = vec![(0.0f32, 0.0f32); pw * lo];
        let q = (ss * ss) as f32;
        for oy in 0..lo {
            for ox in 0..pw {
                let k0 = oy * ss * w + ox * ss;
                let primeiro = id[k0];
                if primeiro == NADA {
                    continue;
                }
                let (mut b1, mut b2, mut igual) = (0.0, 0.0, true);
                'fora: for dy in 0..ss {
                    for dx in 0..ss {
                        let k = k0 + dy * w + dx;
                        if id[k] != primeiro {
                            igual = false;
                            break 'fora;
                        }
                        b1 += bar[k].0;
                        b2 += bar[k].1;
                    }
                }
                if igual {
                    uni[oy * pw + ox] = primeiro;
                    ubar[oy * pw + ox] = (b1 / q, b2 / q);
                }
            }
        }
        let tom = |tid: u32, b: (f32, f32)| {
            let t = &cena.tris[tid as usize];
            let (p, n, u, v) = attr(t, b.0, b.1);
            sombrear(&cena.mats[t.m as usize], p, n, u, v, nivel_de[tid as usize], &luz).0
        };
        // 2) grade de 2 em 2 pixels sombreada de verdade; o miolo das superficies lisas interpola dela (luz varia devagar)
        let mut grade = vec![V3::default(); pw * lo];
        for oy in (0..lo).step_by(2) {
            for ox in (0..pw).step_by(2) {
                let i = oy * pw + ox;
                if uni[i] != NADA {
                    grade[i] = tom(uni[i], ubar[i]);
                }
            }
        }
        let liso = |tid: u32| {
            let m = &cena.mats[cena.tris[tid as usize].m as usize];
            m.tex.is_none() && !m.listras
        };
        let mesma_face = |a: u32, b: u32| {
            a == b || {
                let (ta, tb) = (&cena.tris[a as usize], &cena.tris[b as usize]);
                ta.m == tb.m && ta.v[0].n.dot(tb.v[0].n) > 0.999 && ta.v[1].n.dot(tb.v[1].n) > 0.999
            }
        };
        let mut grupos: Vec<(u32, f32, f32, u32)> = Vec::with_capacity(ss * ss);
        for oy in 0..lo {
            for ox in 0..pw {
                let i = oy * pw + ox;
                let k0 = oy * ss * w + ox * ss;
                let tid = uni[i];
                if tid != NADA {
                    let c = if ox % 2 == 0 && oy % 2 == 0 {
                        grade[i]
                    } else {
                        let (x0, y0) = (ox & !1, oy & !1);
                        let (x1, y1) = (x0 + 2, y0 + 2);
                        let cantos = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)];
                        if liso(tid) && x1 < pw && y1 < lo && cantos.iter().all(|&(x, y)| { let u = uni[y * pw + x]; u != NADA && mesma_face(u, tid) }) {
                            let (fx, fy) = ((ox - x0) as f32 / 2.0, (oy - y0) as f32 / 2.0);
                            let g = |x: usize, y: usize| grade[y * pw + x];
                            (g(x0, y0) * (1.0 - fx) + g(x1, y0) * fx) * (1.0 - fy) + (g(x0, y1) * (1.0 - fx) + g(x1, y1) * fx) * fy
                        } else {
                            tom(tid, ubar[i])
                        }
                    };
                    for dy in 0..ss {
                        for dx in 0..ss {
                            cor[k0 + dy * w + dx] = [c.x, c.y, c.z, 1.0];
                        }
                    }
                    continue;
                }
                // borda: um sombreamento por triangulo presente no pixel
                grupos.clear();
                for dy in 0..ss {
                    for dx in 0..ss {
                        let k = k0 + dy * w + dx;
                        if id[k] == NADA {
                            continue;
                        }
                        match grupos.iter_mut().find(|g| g.0 == id[k]) {
                            Some(g) => {
                                g.1 += bar[k].0;
                                g.2 += bar[k].1;
                                g.3 += 1;
                            }
                            None => grupos.push((id[k], bar[k].0, bar[k].1, 1)),
                        }
                    }
                }
                for g in grupos.iter() {
                    let c = tom(g.0, (g.1 / g.3 as f32, g.2 / g.3 as f32));
                    for dy in 0..ss {
                        for dx in 0..ss {
                            let k = k0 + dy * w + dx;
                            if id[k] == g.0 {
                                cor[k] = [c.x, c.y, c.z, 1.0];
                            }
                        }
                    }
                }
            }
        }
        tp[2] += _t.elapsed().as_secs_f64(); let _t = std::time::Instant::now();
        let mut memo = vec![(NADA, V3::default(), 0.0f32); pw * lo];
        for (i, pr) in &transp {
            let t = &cena.tris[*i];
            let m = &cena.mats[t.m as usize];
            varrer(pr, y_ini, y_fim, w, |k, z, b1, b2| {
                if z >= prof[k] {
                    return;
                }
                let op = (k / w / ss) * pw + (k % w) / ss;
                let (c, a) = if m.tex.is_none() && memo[op].0 == *i as u32 {
                    (memo[op].1, memo[op].2)
                } else {
                    let (p, n, u, v) = attr(t, b1, b2);
                    let r = sombrear(m, p, n, u, v, pr.nivel, &luz);
                    memo[op] = (*i as u32, r.0, r.1);
                    r
                };
                if a <= 0.0 {
                    return;
                }
                let d = &mut cor[k];
                for j in 0..3 {
                    d[j] = [c.x, c.y, c.z][j] * a + d[j] * (1.0 - a);
                }
                d[3] = a + d[3] * (1.0 - a);
            });
        }
        tp[3] += _t.elapsed().as_secs_f64(); let _t = std::time::Instant::now();
        // reduz as ss x ss amostras e compoe por cima do fundo
        let n = (ss * ss) as f32;
        for oy in (y_ini / ss)..(y_fim / ss) {
            for ox in 0..pw {
                let mut s = [0.0f32; 4];
                for dy in 0..ss {
                    for dx in 0..ss {
                        let c = cor[(oy * ss + dy - y_ini) * w + ox * ss + dx];
                        for j in 0..4 {
                            s[j] += c[j];
                        }
                    }
                }
                let a = s[3] / n;
                if a <= 0.0 {
                    continue;
                }
                let i = (oy * pw + ox) * 4;
                for j in 0..3 {
                    dados[i + j] = ((s[j] / n) * 255.0 + dados[i + j] as f32 * (1.0 - a) + 0.5).min(255.0) as u8;
                }
                dados[i + 3] = (a * 255.0 + dados[i + 3] as f32 * (1.0 - a) + 0.5).min(255.0) as u8;
            }
        }
        tp[4] += _t.elapsed().as_secs_f64(); 
        y_ini = y_fim;
    }
    if std::env::var("MOTOR_PERF").is_ok() {
        eprintln!("3d: {} tris | prep {:.0} raster {:.0} sombra {:.0} transp {:.0} reduz {:.0} ms", cena.tris.len(), tp[0] * 1e3, tp[1] * 1e3, tp[2] * 1e3, tp[3] * 1e3, tp[4] * 1e3);
    }
}

// varre um triangulo dentro da faixa [y_ini, y_fim), chamando `f(indice, profundidade, b1, b2)` em cada amostra coberta
#[inline(always)]
fn varrer<F: FnMut(usize, f32, f32, f32)>(pr: &Prep, y_ini: usize, y_fim: usize, w: usize, mut f: F) {
            let ya = pr.y0.max(y_ini as i32);
    let yb = pr.y1.min(y_fim as i32 - 1);
    if ya > yb {
        return;
    }
    let inv = 1.0 / pr.area;
    let (sx, sy) = (pr.sx, pr.sy);
    for py in ya..=yb {
        let cy = py as f32 + 0.5;
        // onde a linha corta as arestas: so esse trecho e testado (os leques das tampas tem caixas enormes)
        let (mut xa, mut xb) = (f32::MAX, f32::MIN);
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            let (ya_, yb_) = (sy[a], sy[b]);
            if (cy - ya_) * (cy - yb_) <= 0.0 && ya_ != yb_ {
                let x = sx[a] + (cy - ya_) * (sx[b] - sx[a]) / (yb_ - ya_);
                xa = xa.min(x);
                xb = xb.max(x);
            }
        }
        if xa > xb {
            continue;
        }
        let xi = ((xa.floor() as i32) - 1).max(pr.x0);
        let xf = ((xb.ceil() as i32) + 1).min(pr.x1);
        for px in xi..=xf {
            let cx = px as f32 + 0.5;
            let w0 = ((sx[2] - sx[1]) * (cy - sy[1]) - (sy[2] - sy[1]) * (cx - sx[1])) * inv;
            let w1 = ((sx[0] - sx[2]) * (cy - sy[2]) - (sy[0] - sy[2]) * (cx - sx[2])) * inv;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let izp = w0 * pr.iz[0] + w1 * pr.iz[1] + w2 * pr.iz[2];
            let z = 1.0 / izp;
            f((py as usize - y_ini) * w + px as usize, z, w1 * pr.iz[1] * z, w2 * pr.iz[2] * z);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn aces_e_srgb_nos_extremos() {
        assert!(aces_srgb(V3::default()).len() < 1e-3);
        let b = aces_srgb(v3(10.0, 10.0, 10.0));
        assert!(b.x > 0.99 && b.y > 0.99);
        assert!((srgb_linear(linear_srgb(0.5)) - 0.5).abs() < 1e-4);
    }
    #[test]
    fn caixa_tem_12_triangulos_de_frente_para_fora() {
        let mut c = Cena3::default();
        let m = c.mat(Mat::std("#ffffff"));
        c.caixa(&M4::id(), 1.0, 1.0, 1.0, m);
        assert_eq!(c.tris.len(), 12);
        for t in &c.tris {
            let f = (t.v[1].p - t.v[0].p).cross(t.v[2].p - t.v[0].p);
            assert!(f.dot(t.v[0].n) > 0.0);
        }
    }
}
