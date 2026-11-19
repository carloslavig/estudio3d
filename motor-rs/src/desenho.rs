// Primitivas: cor, texto (ab_glyph), formas (tiny-skia), mola e interpolacao iguais as do Remotion.
use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use tiny_skia::{Color, FillRule, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Clone, Copy, Debug)]
pub struct Cor(pub f32, pub f32, pub f32, pub f32);

impl Cor {
    pub fn hex(s: &str) -> Cor {
        let s = s.trim_start_matches('#');
        let v = |i: usize| u8::from_str_radix(s.get(i..i + 2).unwrap_or("ff"), 16).unwrap_or(255) as f32 / 255.0;
        Cor(v(0), v(2), v(4), 1.0)
    }
    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Cor {
        Cor(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a)
    }
    pub fn a(self, a: f32) -> Cor {
        Cor(self.0, self.1, self.2, self.3 * a)
    }
    pub fn color(self) -> Color {
        Color::from_rgba(self.0, self.1, self.2, self.3.clamp(0.0, 1.0)).unwrap_or(Color::TRANSPARENT)
    }
    pub fn paint(self) -> Paint<'static> {
        let mut p = Paint::default();
        p.set_color(self.color());
        p.anti_alias = true;
        p
    }
}

/// Nomes de arquivo aceitos para uma fonte pedida pelo nome do Windows, do preferido ao reserva (tudo minusculo).
/// No Mac a Arial e a Courier New existem com outro nome de arquivo; no Linux entram Liberation e DejaVu, de mesma largura.
pub fn candidatas(arquivo: &str) -> Vec<String> {
    const ARIAL: [&str; 4] = ["arial.ttf", "liberationsans-regular.ttf", "dejavusans.ttf", "notosans-regular.ttf"];
    const ARIAL_NEG: [&str; 5] = ["arialbd.ttf", "arial bold.ttf", "liberationsans-bold.ttf", "dejavusans-bold.ttf", "notosans-bold.ttf"];
    const PRETA: [&str; 2] = ["ariblk.ttf", "arial black.ttf"];
    const COUR: [&str; 4] = ["cour.ttf", "courier new.ttf", "liberationmono-regular.ttf", "dejavusansmono.ttf"];
    const COUR_NEG: [&str; 4] = ["courbd.ttf", "courier new bold.ttf", "liberationmono-bold.ttf", "dejavusansmono-bold.ttf"];
    let a = arquivo.to_lowercase();
    let resto: Vec<&str> = match a.as_str() {
        "arial.ttf" | "segoeui.ttf" => ARIAL.to_vec(),
        "arialbd.ttf" | "segoeuib.ttf" => ARIAL_NEG.to_vec(),
        "ariblk.ttf" | "seguibl.ttf" => [&PRETA[..], &ARIAL_NEG[..]].concat(),
        "cour.ttf" => COUR.to_vec(),
        "courbd.ttf" | "consolab.ttf" => COUR_NEG.to_vec(),
        _ => ARIAL.to_vec(),
    };
    let mut v: Vec<String> = resto.into_iter().map(String::from).filter(|x| *x != a).collect();
    v.insert(0, a);
    v
}

/// todas as fontes instaladas (nome do arquivo em minusculo -> caminho), em qualquer sistema.
/// MOTOR_FONTES=pasta (ou varias) entra na frente; uma pasta `fontes` ao lado do programa tambem vale
fn fontes_do_sistema() -> &'static std::collections::HashMap<String, std::path::PathBuf> {
    static F: std::sync::OnceLock<std::collections::HashMap<String, std::path::PathBuf>> = std::sync::OnceLock::new();
    F.get_or_init(|| {
        use std::path::PathBuf;
        let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
        let mut pastas: Vec<PathBuf> = std::env::var_os("MOTOR_FONTES").map(|v| std::env::split_paths(&v).collect()).unwrap_or_default();
        pastas.extend(std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("fontes"))));
        pastas.push(PathBuf::from("fontes"));
        pastas.extend(var("WINDIR").map(|p| p.join("Fonts")));
        pastas.extend(var("LOCALAPPDATA").map(|p| p.join("Microsoft").join("Windows").join("Fonts")));
        for p in ["/System/Library/Fonts", "/Library/Fonts", "/usr/share/fonts", "/usr/local/share/fonts"] {
            pastas.push(PathBuf::from(p));
        }
        for p in ["Library/Fonts", ".fonts", ".local/share/fonts"] {
            pastas.extend(var("HOME").map(|h| h.join(p)));
        }
        let mut mapa = std::collections::HashMap::new();
        // (pasta, quantos niveis ainda desce): as fontes do Linux ficam em subpastas
        let mut fila: Vec<(PathBuf, u8)> = pastas.into_iter().map(|p| (p, 4)).collect();
        fila.reverse();
        while let Some((pasta, nivel)) = fila.pop() {
            for e in std::fs::read_dir(&pasta).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    if nivel > 0 {
                        fila.insert(0, (p, nivel - 1));
                    }
                } else if matches!(p.extension().and_then(|x| x.to_str()).map(|x| x.to_lowercase()).as_deref(), Some("ttf" | "otf")) {
                    if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
                        mapa.entry(n.to_lowercase()).or_insert(p);
                    }
                }
            }
        }
        mapa
    })
}

/// bytes da fonte pedida (nome de arquivo do Windows) ou da equivalente que este sistema tiver
pub fn fonte_bytes(arquivo: &str) -> Result<Vec<u8>, String> {
    let todas = fontes_do_sistema();
    let c = candidatas(arquivo);
    let achada = c.iter().find_map(|n| todas.get(n)).ok_or_else(|| format!("fonte {arquivo}: nao achei nenhuma de {c:?}; instale uma delas ou aponte MOTOR_FONTES para a pasta das fontes"))?;
    std::fs::read(achada).map_err(|e| format!("fonte {}: {e}", achada.display()))
}

#[cfg(test)]
mod teste_fontes {
    #[test]
    fn fonte_do_windows_tem_reserva_no_mac_e_no_linux() {
        let c = super::candidatas("seguibl.ttf");
        assert_eq!(c[0], "seguibl.ttf");
        assert!(c.contains(&"arial black.ttf".to_string()) && c.contains(&"dejavusans-bold.ttf".to_string()));
        assert!(super::candidatas("consolab.ttf").contains(&"courier new bold.ttf".to_string()));
        assert_eq!(super::candidatas("ARIAL.TTF")[0], "arial.ttf");
    }

    #[test]
    fn acha_as_fontes_neste_sistema() {
        for f in ["arial.ttf", "arialbd.ttf", "ariblk.ttf", "cour.ttf", "courbd.ttf", "consolab.ttf", "segoeui.ttf", "seguibl.ttf"] {
            assert!(super::fonte_bytes(f).is_ok_and(|b| b.len() > 1000), "{f}");
        }
    }
}

pub struct Fonte(FontVec);

impl Fonte {
    pub fn abrir(arquivo: &str) -> Result<Fonte, String> {
        FontVec::try_from_vec(fonte_bytes(arquivo)?).map(Fonte).map_err(|e| format!("fonte {arquivo}: {e}"))
    }
    // tamanho CSS (em) -> escala do ab_glyph (que mede ascent-descent)
    fn escala(&self, px: f32) -> PxScale {
        PxScale::from(px * self.0.height_unscaled() / self.0.units_per_em().unwrap_or(1000.0))
    }
    /// (ascendente, descendente negativa, altura de linha "normal")
    pub fn metricas(&self, px: f32) -> (f32, f32, f32) {
        let s = self.0.as_scaled(self.escala(px));
        (s.ascent(), s.descent(), s.ascent() - s.descent() + s.line_gap())
    }
    /// largura com letter-spacing (o CSS soma o espacamento depois de cada letra)
    pub fn largura(&self, px: f32, txt: &str, esp: f32) -> f32 {
        let s = self.0.as_scaled(self.escala(px));
        let mut w = 0.0;
        let mut ant = None;
        for c in txt.chars() {
            let id = self.0.glyph_id(c);
            if let Some(a) = ant {
                w += s.kern(a, id);
            }
            w += s.h_advance(id) + esp;
            ant = Some(id);
        }
        w
    }
    /// baseline de uma linha cuja caixa comeca em `topo` e tem altura `lh` (como o CSS centraliza)
    pub fn baseline(&self, px: f32, topo: f32, lh: f32) -> f32 {
        let (asc, desc, _) = self.metricas(px);
        topo + (lh - (asc - desc)) / 2.0 + asc
    }
    pub fn desenhar(&self, pm: &mut Pixmap, px: f32, x: f32, baseline: f32, txt: &str, esp: f32, cor: Cor) {
        if cor.3 <= 0.0 || px < 1.0 {
            return;
        }
        let escala = self.escala(px);
        let s = self.0.as_scaled(escala);
        let (w, h) = (pm.width() as i32, pm.height() as i32);
        let dados = pm.data_mut();
        let mut cx = x;
        let mut ant = None;
        for c in txt.chars() {
            let id = self.0.glyph_id(c);
            if let Some(a) = ant {
                cx += s.kern(a, id);
            }
            let g = id.with_scale_and_position(escala, ab_glyph::point(cx, baseline));
            if let Some(og) = self.0.outline_glyph(g) {
                let b = og.px_bounds();
                og.draw(|gx, gy, cov| {
                    let (px_, py_) = (b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32);
                    if px_ < 0 || py_ < 0 || px_ >= w || py_ >= h {
                        return;
                    }
                    misturar(dados, ((py_ * w + px_) * 4) as usize, cor, cov.min(1.0));
                });
            }
            cx += s.h_advance(id) + esp;
            ant = Some(id);
        }
    }
    /// text-shadow "0 0 Bpx cor": desenha o texto num rascunho, desfoca (3 box blurs ~ gaussiano) e aplica por baixo
    pub fn brilho(&self, pm: &mut Pixmap, px: f32, x: f32, baseline: f32, txt: &str, esp: f32, cor: Cor, desfoque: f32) {
        if cor.3 <= 0.0 || px < 1.0 {
            return;
        }
        let sigma = desfoque / 2.0;
        let pad = (sigma * 3.0).ceil();
        let (asc, desc, _) = self.metricas(px);
        let (w, h) = ((self.largura(px, txt, esp) + 2.0 * pad).ceil() as usize, (asc - desc + 2.0 * pad).ceil() as usize);
        let Some(mut rasc) = Pixmap::new(w as u32, h as u32) else { return };
        self.desenhar(&mut rasc, px, pad, pad + asc, txt, esp, Cor(1.0, 1.0, 1.0, 1.0));
        let mut a: Vec<f32> = rasc.data().chunks_exact(4).map(|p| p[3] as f32 / 255.0).collect();
        let r = ((sigma * sigma * 4.0 + 1.0).sqrt() / 2.0).round().max(1.0) as usize; // raio de cada passada
        let mut tmp = vec![0.0f32; a.len()];
        for _ in 0..3 {
            caixa(&a, &mut tmp, w, h, r, true);
            caixa(&tmp, &mut a, w, h, r, false);
        }
        let (ox, oy) = ((x - pad).round() as i32, (baseline - asc - pad).round() as i32);
        let (pw, ph) = (pm.width() as i32, pm.height() as i32);
        let dados = pm.data_mut();
        for yy in 0..h as i32 {
            for xx in 0..w as i32 {
                let (px_, py_) = (ox + xx, oy + yy);
                if px_ >= 0 && py_ >= 0 && px_ < pw && py_ < ph {
                    misturar(dados, ((py_ * pw + px_) * 4) as usize, cor, a[yy as usize * w + xx as usize]);
                }
            }
        }
    }
    /// quebra por palavras como o navegador (greedy)
    pub fn quebrar(&self, px: f32, txt: &str, esp: f32, max: f32) -> Vec<String> {
        let mut linhas = Vec::new();
        let mut atual = String::new();
        for w in txt.split_whitespace() {
            let tentativa = if atual.is_empty() { w.to_string() } else { format!("{atual} {w}") };
            if !atual.is_empty() && self.largura(px, &tentativa, esp) > max {
                linhas.push(std::mem::replace(&mut atual, w.to_string()));
            } else {
                atual = tentativa;
            }
        }
        if !atual.is_empty() {
            linhas.push(atual);
        }
        linhas
    }
}

// media movel de largura 2r+1 numa direcao (soma corrida)
fn caixa(de: &[f32], para: &mut [f32], w: usize, h: usize, r: usize, horizontal: bool) {
    let (n, linhas, passo, salto) = if horizontal { (w, h, 1, w) } else { (h, w, w, 1) };
    let div = (2 * r + 1) as f32;
    for l in 0..linhas {
        let base = l * salto;
        let at = |i: isize| if i < 0 || i >= n as isize { 0.0 } else { de[base + i as usize * passo] };
        let mut soma: f32 = (-(r as isize)..=r as isize).map(at).sum();
        for i in 0..n {
            para[base + i * passo] = soma / div;
            soma += at(i as isize + r as isize + 1) - at(i as isize - r as isize);
        }
    }
}

// src-over em RGBA pre-multiplicado
#[inline]
fn misturar(d: &mut [u8], i: usize, cor: Cor, cov: f32) {
    let a = cor.3 * cov;
    if a <= 0.0 {
        return;
    }
    let inv = 1.0 - a;
    d[i] = (cor.0 * 255.0 * a + d[i] as f32 * inv + 0.5) as u8;
    d[i + 1] = (cor.1 * 255.0 * a + d[i + 1] as f32 * inv + 0.5) as u8;
    d[i + 2] = (cor.2 * 255.0 * a + d[i + 2] as f32 * inv + 0.5) as u8;
    d[i + 3] = (255.0 * a + d[i + 3] as f32 * inv + 0.5) as u8;
}

pub fn caminho_ret(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let k = 0.5523 * r;
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

pub fn ret(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, cor: Cor) {
    if let Some(p) = caminho_ret(x, y, w, h, r) {
        pm.fill_path(&p, &cor.paint(), FillRule::Winding, Transform::identity(), None);
    }
}

/// borda CSS (fica por dentro da caixa)
pub fn borda(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, cor: Cor, larg: f32) {
    let m = larg / 2.0;
    if let Some(p) = caminho_ret(x + m, y + m, w - larg, h - larg, r - m) {
        let st = Stroke { width: larg, ..Default::default() };
        pm.stroke_path(&p, &cor.paint(), &st, Transform::identity(), None);
    }
}

pub fn circulo(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, cor: Cor) {
    if let Some(p) = PathBuilder::from_circle(cx, cy, r) {
        pm.fill_path(&p, &cor.paint(), FillRule::Winding, Transform::identity(), None);
    }
}

/// spring() do Remotion (massa 1, rigidez 100), em forma fechada
pub fn mola(quadro: f32, fps: f32, amortecimento: f32) -> f32 {
    if quadro <= 0.0 {
        return 0.0;
    }
    let t = quadro / fps;
    let w0 = 10.0f32;
    let z = amortecimento / 20.0;
    if z < 1.0 {
        let wd = w0 * (1.0 - z * z).sqrt();
        1.0 - (-z * w0 * t).exp() * ((wd * t).cos() + z * w0 / wd * (wd * t).sin())
    } else {
        1.0 - (-w0 * t).exp() * (1.0 + w0 * t)
    }
}

/// interpolate() do Remotion com clamp nas duas pontas
pub fn interp(x: f32, de: &[f32], para: &[f32]) -> f32 {
    if x <= de[0] {
        return para[0];
    }
    for i in 1..de.len() {
        if x <= de[i] {
            let f = (x - de[i - 1]) / (de[i] - de[i - 1]).max(1e-9);
            return para[i - 1] + (para[i] - para[i - 1]) * f;
        }
    }
    para[para.len() - 1]
}

/// toLocaleString("pt-BR") com casas fixas
pub fn br(v: f64, casas: usize) -> String {
    let s = format!("{:.*}", casas, v.abs());
    let (int, dec) = s.split_once('.').unwrap_or((&s, ""));
    let mut g = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            g.push('.');
        }
        g.push(c);
    }
    let sinal = if v < 0.0 && s.chars().any(|c| c.is_ascii_digit() && c != '0') { "-" } else { "" };
    if dec.is_empty() { format!("{sinal}{g}") } else { format!("{sinal}{g},{dec}") }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn numero_em_pt_br() {
        assert_eq!(br(12_400_000.0, 0), "12.400.000");
        assert_eq!(br(1234.5, 1), "1.234,5");
        assert_eq!(br(999.0, 2), "999,00");
        assert_eq!(br(-1500.0, 0), "-1.500");
        assert_eq!(br(0.0, 0), "0");
    }
    #[test]
    fn mola_e_interp() {
        assert_eq!(mola(0.0, 30.0, 14.0), 0.0);
        assert!((mola(90.0, 30.0, 14.0) - 1.0).abs() < 0.01);
        assert_eq!(interp(-1.0, &[4.0, 40.0], &[0.0, 1.0]), 0.0);
        assert_eq!(interp(22.0, &[4.0, 40.0], &[0.0, 1.0]), 0.5);
        assert_eq!(interp(99.0, &[4.0, 40.0], &[0.0, 1.0]), 1.0);
    }
}
