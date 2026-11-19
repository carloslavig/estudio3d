// Peca MAPA (src/pecas/Mapa.tsx) no motor-rs: satelite aberto + divisas do IBGE em Mercator, zoom Brasil -> estado -> municipio.
// Dados vem de tools/geo.py e tools/sat.py (public/geo/<uf>-<slug>.json e .sat.json). Carregados uma vez (cache estatico).
use crate::desenho::{borda, interp, ret, Cor};
use ab_glyph::{Font, FontVec, OutlineCurve};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tiny_skia::{FillRule, FilterQuality, IntSize, Path as Caminho, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform};

/// raiz do Estudio3D, definida uma vez no main (o Motor nao guarda a raiz)
pub static RAIZ: OnceLock<PathBuf> = OnceLock::new();
pub fn raiz() -> &'static Path {
    RAIZ.get().map(PathBuf::as_path).unwrap_or(Path::new("."))
}

const ESC: f32 = 1000.0; // Mercator normalizado x 1000, como o SVG do Remotion
const FPS: f32 = 30.0;

#[derive(Clone, Copy)]
struct Caixa {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}
struct Feat {
    cod: String,
    aneis: Vec<Vec<(f32, f32)>>,
    caixa: Caixa,
}
struct Nivel {
    mips: Vec<Pixmap>, // [0] = original, cada um a metade do anterior (evita serrilhado ao reduzir)
    c: Caixa,
}
struct Geo {
    estados: Vec<Feat>,
    municipios: Vec<Feat>,
    cod_uf: String,
    nome_uf: String,
    alvo_cod: String,
    alvo_nome: String,
    sat: Option<(Vec<Nivel>, String)>, // brasil, estado, cidade + credito
    perto: Vec<Nivel>,                 // visual "magno": mosaicos extras em volta do quadro final (tools/sat.py --perto)
}

fn pj(lon: f64, lat: f64) -> (f32, f32) {
    let l = lat.clamp(-85.05, 85.05);
    let y = 0.5 - (std::f64::consts::FRAC_PI_4 + l.to_radians() / 2.0).tan().ln() / (2.0 * std::f64::consts::PI);
    ((((lon + 180.0) / 360.0) * ESC as f64) as f32, (y * ESC as f64) as f32)
}

fn slug(t: &str) -> String {
    let mut s = String::new();
    for c in t.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let s: String = s.trim_matches('-').chars().take(40).collect();
    if s.is_empty() { "x".into() } else { s }
}

fn caixa_de<'a>(fs: impl IntoIterator<Item = &'a Feat>) -> Caixa {
    let mut c = Caixa { x0: 1e9, y0: 1e9, x1: -1e9, y1: -1e9 };
    for f in fs {
        c.x0 = c.x0.min(f.caixa.x0);
        c.y0 = c.y0.min(f.caixa.y0);
        c.x1 = c.x1.max(f.caixa.x1);
        c.y1 = c.y1.max(f.caixa.y1);
    }
    c
}
fn enquadrar(c: Caixa, prop: f32, folga: f32) -> Caixa {
    let (cx, cy) = ((c.x0 + c.x1) / 2.0, (c.y0 + c.y1) / 2.0);
    let (mut w, mut h) = ((c.x1 - c.x0) * (1.0 + folga), (c.y1 - c.y0) * (1.0 + folga));
    if w / h < prop { w = h * prop } else { h = w / prop }
    Caixa { x0: cx - w / 2.0, y0: cy - h / 2.0, x1: cx + w / 2.0, y1: cy + h / 2.0 }
}
fn mistura(a: Caixa, b: Caixa, t: f32) -> Caixa {
    Caixa { x0: a.x0 + (b.x0 - a.x0) * t, y0: a.y0 + (b.y0 - a.y0) * t, x1: a.x1 + (b.x1 - a.x1) * t, y1: a.y1 + (b.y1 - a.y1) * t }
}
fn suave(f: f32, de: f32, ate: f32) -> f32 {
    let t = interp(f, &[de, ate], &[0.0, 1.0]);
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

// ---------- carga (uma vez por uf+municipio) ----------

fn feats(v: &Value) -> Vec<Feat> {
    let mut out = Vec::new();
    for f in v["features"].as_array().into_iter().flatten() {
        let cod = match &f["properties"]["cod"] {
            Value::String(s) => s.clone(),
            v => v.to_string(),
        };
        let g = &f["geometry"];
        let polis: Vec<&Value> = if g["type"] == "Polygon" { vec![&g["coordinates"]] } else { g["coordinates"].as_array().map(|a| a.iter().collect()).unwrap_or_default() };
        let mut aneis = Vec::new();
        for p in polis {
            for anel in p.as_array().into_iter().flatten() {
                let pts: Vec<(f32, f32)> = anel.as_array().into_iter().flatten().filter_map(|q| Some(pj(q[0].as_f64()?, q[1].as_f64()?))).collect();
                if pts.len() > 1 {
                    aneis.push(pts);
                }
            }
        }
        let mut c = Caixa { x0: 1e9, y0: 1e9, x1: -1e9, y1: -1e9 };
        for (x, y) in aneis.iter().flatten() {
            c.x0 = c.x0.min(*x);
            c.y0 = c.y0.min(*y);
            c.x1 = c.x1.max(*x);
            c.y1 = c.y1.max(*y);
        }
        out.push(Feat { cod, aneis, caixa: c });
    }
    out
}

fn imagem(caminho: &Path) -> Option<Vec<Pixmap>> {
    let mut img = image::open(caminho).ok()?.to_rgba8(); // jpg opaco: RGBA ja e "pre-multiplicado"
    let mut mips = Vec::new();
    loop {
        let (w, h) = img.dimensions();
        mips.push(Pixmap::from_vec(img.clone().into_raw(), IntSize::from_wh(w, h)?)?);
        if w < 64 || h < 64 {
            break;
        }
        img = image::imageops::resize(&img, w / 2, h / 2, image::imageops::FilterType::Triangle);
    }
    Some(mips)
}

fn ler(raiz: &Path, base: &str) -> Option<Geo> {
    let pub_ = raiz.join("public");
    let g: Value = serde_json::from_str(&std::fs::read_to_string(pub_.join(format!("{base}.json"))).ok()?).ok()?;
    let sat = std::fs::read_to_string(pub_.join(format!("{base}.sat.json"))).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|s| {
        let mut niveis = Vec::new();
        for k in ["brasil", "estado", "cidade"] {
            let n = &s["niveis"][k];
            let c = Caixa { x0: n["x0"].as_f64()? as f32, y0: n["y0"].as_f64()? as f32, x1: n["x1"].as_f64()? as f32, y1: n["y1"].as_f64()? as f32 };
            niveis.push(Nivel { mips: imagem(&pub_.join(n["arquivo"].as_str()?))?, c });
        }
        Some((niveis, s["credito"].as_str().unwrap_or("").to_string()))
    });
    let txt = |v: &Value| match v {
        Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    let perto: Vec<Nivel> = std::fs::read_to_string(pub_.join(format!("{base}.sat-perto.json")))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|s| s["niveis"].as_object().cloned())
        .into_iter()
        .flatten()
        .filter_map(|(_, n)| {
            let c = Caixa { x0: n["x0"].as_f64()? as f32, y0: n["y0"].as_f64()? as f32, x1: n["x1"].as_f64()? as f32, y1: n["y1"].as_f64()? as f32 };
            Some(Nivel { mips: imagem(&pub_.join(n["arquivo"].as_str()?))?, c })
        })
        .collect();
    Some(Geo {
        perto,
        estados: feats(&g["estados"]),
        municipios: feats(&g["municipios"]),
        cod_uf: txt(&g["cod_uf"]),
        nome_uf: txt(&g["nome_uf"]),
        alvo_cod: txt(&g["alvo"]["cod"]),
        alvo_nome: txt(&g["alvo"]["nome"]),
        sat,
    })
}

fn carregar(raiz: &Path, uf: &str, municipio: &str) -> Option<Arc<Geo>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<Geo>>>>> = OnceLock::new();
    let base = format!("geo/{}-{}", uf.to_lowercase(), slug(municipio));
    let mut m = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    m.entry(base.clone())
        .or_insert_with(|| {
            let g = ler(raiz, &base).map(Arc::new);
            if g.is_none() {
                eprintln!("mapa sem dados: {base}.json (rode tools/geo.py e tools/sat.py)");
            }
            g
        })
        .clone()
}

// ---------- texto como caminho (para ter contorno, como o SVG com paint-order stroke) ----------

struct Fontes {
    mono: FontVec,
    sans: FontVec,
    negrito: FontVec, // visual "magno": nome da cidade
    preto: FontVec,   // visual "magno": plaquinhas, faixa e etiqueta (letra larga e pesada, como no video)
}
fn fontes() -> &'static Fontes {
    static F: OnceLock<Fontes> = OnceLock::new();
    F.get_or_init(|| {
        let abrir = |a: &str| FontVec::try_from_vec(crate::desenho::fonte_bytes(a).expect("fonte do mapa")).expect("fonte do mapa");
        // shortcut: pinos e legendas usam Segoe UI (fonte do estilo "mesa"); passar a fonte do estilo se outro estilo usar mapa
        Fontes { mono: abrir("courbd.ttf"), sans: abrir("segoeui.ttf"), negrito: abrir("arialbd.ttf"), preto: abrir("ariblk.ttf") }
    })
}
fn k_de(f: &FontVec, px: f32) -> f32 {
    px / f.units_per_em().unwrap_or(1000.0)
}
fn largura(f: &FontVec, px: f32, txt: &str, esp: f32) -> f32 {
    let k = k_de(f, px);
    let mut w = 0.0;
    let mut ant = None;
    for c in txt.chars() {
        let id = f.glyph_id(c);
        if let Some(a) = ant {
            w += f.kern_unscaled(a, id) * k;
        }
        w += f.h_advance_unscaled(id) * k + esp;
        ant = Some(id);
    }
    w
}
/// (altura de linha "normal", baseline dentro de uma caixa dessa altura)
fn linha(f: &FontVec, px: f32) -> (f32, f32) {
    let k = k_de(f, px);
    let (a, d) = (f.ascent_unscaled() * k, f.descent_unscaled() * k);
    let lh = a - d + f.line_gap_unscaled() * k;
    (lh, (lh - (a - d)) / 2.0 + a)
}
fn texto(f: &FontVec, px: f32, x: f32, base: f32, txt: &str, esp: f32) -> Option<Caminho> {
    let k = k_de(f, px);
    let mut pb = PathBuilder::new();
    let mut cx = x;
    let mut ant = None;
    for c in txt.chars() {
        let id = f.glyph_id(c);
        if let Some(a) = ant {
            cx += f.kern_unscaled(a, id) * k;
        }
        if let Some(o) = f.outline(id) {
            let ox = cx;
            let p = |q: ab_glyph::Point| (ox + q.x * k, base - q.y * k);
            let mut fim: Option<ab_glyph::Point> = None;
            for cur in &o.curves {
                let ini = match cur {
                    OutlineCurve::Line(a, _) | OutlineCurve::Quad(a, _, _) | OutlineCurve::Cubic(a, _, _, _) => *a,
                };
                if fim != Some(ini) {
                    if fim.is_some() {
                        pb.close();
                    }
                    let (x0, y0) = p(ini);
                    pb.move_to(x0, y0);
                }
                match cur {
                    OutlineCurve::Line(_, b) => {
                        let (x1, y1) = p(*b);
                        pb.line_to(x1, y1);
                        fim = Some(*b);
                    }
                    OutlineCurve::Quad(_, b, c) => {
                        let ((x1, y1), (x2, y2)) = (p(*b), p(*c));
                        pb.quad_to(x1, y1, x2, y2);
                        fim = Some(*c);
                    }
                    OutlineCurve::Cubic(_, b, c, d) => {
                        let ((x1, y1), (x2, y2), (x3, y3)) = (p(*b), p(*c), p(*d));
                        pb.cubic_to(x1, y1, x2, y2, x3, y3);
                        fim = Some(*d);
                    }
                }
            }
            if fim.is_some() {
                pb.close();
            }
        }
        cx += f.h_advance_unscaled(id) * k + esp;
        ant = Some(id);
    }
    pb.finish()
}

// ---------- pintura ----------

fn encher(pm: &mut Pixmap, p: &Caminho, cor: Cor) {
    if cor.3 > 0.002 {
        pm.fill_path(p, &cor.paint(), FillRule::Winding, Transform::identity(), None);
    }
}
fn tracar(pm: &mut Pixmap, p: &Caminho, cor: Cor, larg: f32) {
    if cor.3 > 0.002 {
        let st = Stroke { width: larg, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
        pm.stroke_path(p, &cor.paint(), &st, Transform::identity(), None);
    }
}
/// texto com contorno embaixo (paint-order: stroke), opacidade aplicada ao grupo
fn texto_contorno(pm: &mut Pixmap, f: &FontVec, px: f32, x: f32, base: f32, txt: &str, esp: f32, contorno: f32, op: f32) {
    if op <= 0.0 {
        return;
    }
    if let Some(p) = texto(f, px, x, base, txt, esp) {
        tracar(pm, &p, Cor(0.0, 0.0, 0.0, op), contorno);
        encher(pm, &p, Cor(1.0, 1.0, 1.0, op));
    }
}

fn borrar(a: &mut [f32], w: usize, h: usize, sigma: f32) {
    let r = ((4.0 * sigma * sigma + 1.0).sqrt() / 2.0).round().max(1.0) as usize;
    let mut t = vec![0.0f32; a.len()];
    let passada = |de: &[f32], para: &mut [f32], horiz: bool| {
        let (n, linhas, passo, salto) = if horiz { (w, h, 1, w) } else { (h, w, w, 1) };
        let div = (2 * r + 1) as f32;
        for l in 0..linhas {
            let base = l * salto;
            let at = |i: isize| if i < 0 || i >= n as isize { 0.0 } else { de[base + i as usize * passo] };
            let mut s: f32 = (-(r as isize)..=r as isize).map(at).sum();
            for i in 0..n {
                para[base + i * passo] = s / div;
                s += at(i as isize + r as isize + 1) - at(i as isize - r as isize);
            }
        }
    };
    for _ in 0..3 {
        passada(a, &mut t, true);
        passada(&t, a, false);
    }
}

/// filter: drop-shadow(dx dy B cor): borra o alfa de `sombra`, pinta a sombra e depois `forma` por cima.
/// Medido contra o Chrome (13/11): no preenchimento semitransparente da cidade a sombra nao aparece por dentro, so o brilho do contorno;
/// por isso a sombra pode vir de outra forma (so o traco).
fn com_sombra(pm: &mut Pixmap, area: (f32, f32, f32, f32), dx: f32, dy: f32, sigma: f32, cor: Cor, sombra: &dyn Fn(&mut Pixmap, Transform), forma: &dyn Fn(&mut Pixmap, Transform)) {
    let pad = (sigma * 3.0).ceil() + 2.0;
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let x0 = (area.0 - pad).max(0.0).floor().min(w);
    let y0 = (area.1 - pad).max(0.0).floor().min(h);
    let x1 = (area.2 + pad).min(w).ceil();
    let y1 = (area.3 + pad).min(h).ceil();
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let (rw, rh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let Some(mut rasc) = Pixmap::new(rw as u32, rh as u32) else { return };
    let tf = Transform::from_translate(-x0, -y0);
    sombra(&mut rasc, tf);
    if cor.3 > 0.0 {
        let mut a: Vec<f32> = rasc.data().chunks_exact(4).map(|p| p[3] as f32 / 255.0).collect();
        borrar(&mut a, rw, rh, sigma);
        let (pw, ph) = (pm.width() as i32, pm.height() as i32);
        let d = pm.data_mut();
        for y in 0..rh {
            for x in 0..rw {
                let (px, py) = (x0 as i32 + x as i32 + dx.round() as i32, y0 as i32 + y as i32 + dy.round() as i32);
                let al = a[y * rw + x] * cor.3;
                if al <= 0.0 || px < 0 || py < 0 || px >= pw || py >= ph {
                    continue;
                }
                let i = ((py * pw + px) * 4) as usize;
                let inv = 1.0 - al;
                d[i] = (cor.0 * 255.0 * al + d[i] as f32 * inv + 0.5) as u8;
                d[i + 1] = (cor.1 * 255.0 * al + d[i + 1] as f32 * inv + 0.5) as u8;
                d[i + 2] = (cor.2 * 255.0 * al + d[i + 2] as f32 * inv + 0.5) as u8;
                d[i + 3] = (255.0 * al + d[i + 3] as f32 * inv + 0.5) as u8;
            }
        }
    }
    forma(pm, Transform::identity());
}

// ---------- a peca ----------

/// Desenha a peca "mapa". lf = quadro local da cena, dur = duracao da cena em quadros.
/// verde (0..1, so magno): a cidade fica mais verde no fim prolongado do mapa ("arrecadou").
pub fn desenhar(pm: &mut Pixmap, dados: &Value, lf: f32, dur: f32, raiz: &Path, cor: Cor, texto_cor: Cor, magno: bool, verde: f32) {
    let uf = dados["uf"].as_str().unwrap_or("BA").to_uppercase();
    let municipio = dados["municipio"].as_str().unwrap_or("");
    let Some(g) = carregar(raiz, &uf, municipio) else { return };
    let cheia = dados["tela_cheia"].as_bool().unwrap_or(false); // padrao Magno: satelite na tela inteira
    let so_estado = dados["zoom_final"].as_str() == Some("estado");
    let (l, a) = if cheia { (1080.0f32, 1920.0f32) } else { (960.0, 900.0) };
    let frame = lf;
    let estado_alvo: Vec<&Feat> = g.estados.iter().filter(|f| f.cod == g.cod_uf).collect();
    let cidade: Vec<&Feat> = g.municipios.iter().filter(|f| f.cod == g.alvo_cod).collect();
    if cidade.is_empty() || estado_alvo.is_empty() {
        return;
    }
    let prop = l / a;
    let v0 = enquadrar(caixa_de(&g.estados), prop, 0.04);
    let v1 = enquadrar(caixa_de(estado_alvo.iter().copied()), prop, 0.1);
    let cc = caixa_de(cidade.iter().copied());
    let v2 = enquadrar(cc, prop, 0.7);
    let me = dados["marcas"]["estado"].as_f64().map(|s| s as f32 * FPS).unwrap_or(dur * 0.38);
    let mc = dados["marcas"]["cidade"].as_f64().map(|s| (s as f32 * FPS).max(me + 1.4 * FPS)).unwrap_or(dur * 0.72);
    let e1 = suave(frame, me - 0.9 * FPS, me + 0.6 * FPS);
    let e2 = if so_estado { 0.0 } else { suave(frame, mc - 0.9 * FPS, mc + 0.6 * FPS) };
    let mg = magno && cheia; // visual "magno": voo continuo ate bem perto da cidade, sem destaque do estado
    // magno: o quadro inicial cabe dentro do mosaico do Brasil (senao sobra faixa escura em cima, que o video nao tem)
    let v0m = match (&g.sat, mg) {
        (Some((niveis, _)), true) => caber(v0, Caixa { x0: niveis[0].c.x0 * ESC, y0: niveis[0].c.y0 * ESC, x1: niveis[0].c.x1 * ESC, y1: niveis[0].c.y1 * ESC }),
        _ => v0,
    };
    let (v, z) = if mg { voo(v0m, cc, prop, frame / FPS) } else { (mistura(mistura(v0, v1, e1), v2, e2), 0.0) };
    let lv = v.x1 - v.x0;
    let s = l / lv; // pixels por unidade do SVG
    let tela = |x: f32, y: f32| ((x - v.x0) * s, (y - v.y0) * s);
    let centro = tela((cc.x0 + cc.x1) / 2.0, (cc.y0 + cc.y1) / 2.0);
    let pulso = 1.0 + 0.25 * (frame * 0.25).sin();
    let marcador = interp(frame, &[me - 0.2 * FPS, me + 0.8 * FPS], &[0.0, 1.0]);
    let entrada = interp(frame, &[0.0, 14.0], &[0.0, 1.0]);
    let com_sat = g.sat.is_some();
    let fo = fontes();
    // drop-shadow dentro do SVG: o desfoque esta em unidades do SVG (escala com o zoom), conferido contra o video do Chrome.
    // Acima de ~60 px de tela o brilho some (fica abaixo de 3% de opacidade), entao nem desenha.
    let sigma_svg = |b: f32| { let sg = b / 2.0 * s; if sg > 60.0 { 0.0 } else { sg } };

    let caminho_de = |f: &Feat| -> Option<Caminho> {
        let (x0, y0) = tela(f.caixa.x0, f.caixa.y0);
        let (x1, y1) = tela(f.caixa.x1, f.caixa.y1);
        if x1 < -20.0 || y1 < -20.0 || x0 > l + 20.0 || y0 > a + 20.0 {
            return None; // fora da tela
        }
        let mut pb = PathBuilder::new();
        for anel in &f.aneis {
            for (i, (x, y)) in anel.iter().enumerate() {
                let (px, py) = tela(*x, *y);
                if i == 0 { pb.move_to(px, py) } else { pb.line_to(px, py) }
            }
            pb.close();
        }
        pb.finish()
    };
    let limites = |p: &Caminho, m: f32| {
        let b = p.bounds();
        (b.left() - m, b.top() - m, b.right() + m, b.bottom() + m)
    };

    // a "folha" do SVG: tela cheia direto na camada; carta num rascunho que depois e inclinado
    let mut camada = Pixmap::new(1080, 1920).expect("pixmap");
    let mut forca = 0.0f32; // visual "magno": quanto da correcao de cor do satelite entra
    let mut carta = if cheia { None } else { Pixmap::new(960, 900) };
    {
        let mp: &mut Pixmap = match carta.as_mut() {
            Some(c) => c,
            None => &mut camada,
        };
        mp.fill(Cor::hex("#0a1a33").color());
        if let Some((niveis, _)) = &g.sat {
            // magno: todos os mosaicos (do mais largo ao mais fino); cada um aparece quando cobre a vista inteira
            let mut todos: Vec<&Nivel> = niveis.iter().collect();
            if mg {
                todos.extend(g.perto.iter());
                todos.sort_by(|a, b| (b.c.x1 - b.c.x0).total_cmp(&(a.c.x1 - a.c.x0)));
            }
            let ops: Vec<f32> = if mg {
                todos
                    .iter()
                    .map(|n| {
                        let (x0, y0, x1, y1) = (n.c.x0 * ESC, n.c.y0 * ESC, n.c.x1 * ESC, n.c.y1 * ESC);
                        let folga = (v.x0 - x0).min(x1 - v.x1).min(v.y0 - y0).min(y1 - v.y1) / lv;
                        interp(folga, &[-0.03, 0.0], &[0.0, 1.0])
                    })
                    .collect()
            } else {
                vec![1.0, e1, e2]
            };
            // a correcao de cor foi medida no Sentinel (de perto); no Brasil inteiro (NASA) ela entra junto com os mosaicos finos
            forca = ops.iter().skip(1).cloned().fold(0.0, f32::max);
            for (i, (n, o)) in todos.into_iter().zip(ops).enumerate() {
                let o = if i == 0 { 1.0 } else { o };
                if o <= 0.01 {
                    continue;
                }
                let (tx, ty) = tela(n.c.x0 * ESC, n.c.y0 * ESC);
                let larg_tela = (n.c.x1 - n.c.x0) * ESC * s;
                // nivel de mip: o menor que ainda tem pelo menos a largura desenhada
                let img = n.mips.iter().rev().find(|m| m.width() as f32 >= larg_tela).unwrap_or(&n.mips[0]);
                let (sx, sy) = (larg_tela / img.width() as f32, (n.c.y1 - n.c.y0) * ESC * s / img.height() as f32);
                let paint = PixmapPaint { opacity: o, quality: FilterQuality::Bilinear, ..Default::default() };
                mp.draw_pixmap(0, 0, img.as_ref(), &paint, Transform::from_row(sx, 0.0, 0.0, sy, tx, ty), None);
            }
        }
        if mg && forca > 0.0 {
            graduar(mp, forca);
        }
        for f in &g.estados {
            let Some(p) = caminho_de(f) else { continue };
            if mg {
                tracar(mp, &p, Cor(1.0, 1.0, 1.0, 0.5 * interp(z, &[0.25, 0.5], &[1.0, 0.0])), 1.3);
                continue;
            }
            if f.cod == g.cod_uf {
                // estado alvo: contorno na cor + drop-shadow 8px
                let st = Stroke { width: 3.2, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
                let traco = |r: &mut Pixmap, tf: Transform| r.stroke_path(&p, &cor.paint(), &st, tf, None);
                let tudo = |r: &mut Pixmap, tf: Transform| {
                    if !com_sat {
                        r.fill_path(&p, &cor.a(0.35).paint(), FillRule::Winding, tf, None);
                    }
                    traco(r, tf);
                };
                let sg = sigma_svg(8.0);
                if sg > 0.0 { com_sombra(mp, limites(&p, 3.0), 0.0, 0.0, sg, cor, &traco, &tudo) } else { tudo(mp, Transform::identity()) }
            } else {
                let fundo = if com_sat { Cor::hex("#06142a").a(0.5 * e1) } else { Cor::hex("#2a4a9a").a(0.55 * (1.0 - 0.85 * e1)) };
                encher(mp, &p, fundo);
                tracar(mp, &p, Cor(1.0, 1.0, 1.0, 0.65), 1.3);
            }
        }
        let op_mun = 0.4 * e1 * (1.0 - 0.4 * e2);
        if op_mun > 0.004 && !mg {
            for f in g.municipios.iter().filter(|f| f.cod != g.alvo_cod) {
                if let Some(p) = caminho_de(f) {
                    tracar(mp, &p, Cor(1.0, 1.0, 1.0, op_mun), 0.9);
                }
            }
        }
        for f in cidade.iter().filter(|_| mg) {
            // magno: contorno claro e preenchimento leve esverdeado, entram quando a cidade ja aparece
            let Some(p) = caminho_de(f) else { continue };
            let k = interp(z, &[0.55, 0.8], &[0.0, 1.0]);
            encher(mp, &p, Cor(0.82, 1.0, 0.9, 0.3 * k));
            encher(mp, &p, Cor::hex("#2fbf8f").a(0.55 * verde));
            let st = Stroke { width: 3.0, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
            let traco = |r: &mut Pixmap, tf: Transform| r.stroke_path(&p, &Cor(0.88, 0.97, 1.0, k).paint(), &st, tf, None);
            if k > 0.0 {
                com_sombra(mp, limites(&p, 3.0), 0.0, 0.0, 4.0, Cor(0.0, 0.0, 0.0, 0.5 * k), &traco, &traco)
            }
        }
        for f in cidade.iter().filter(|_| !mg) {
            let Some(p) = caminho_de(f) else { continue };
            let preenche = cor.a(if com_sat { 0.1 + 0.15 * e1 } else { 0.35 + 0.65 * e1 });
            let larg = if e2 > 0.0 { 4.0 } else { 2.4 };
            let st = Stroke { width: larg, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
            let traco = |r: &mut Pixmap, tf: Transform| r.stroke_path(&p, &Cor(1.0, 1.0, 1.0, 1.0).paint(), &st, tf, None);
            let tudo = |r: &mut Pixmap, tf: Transform| {
                r.fill_path(&p, &preenche.paint(), FillRule::Winding, tf, None);
                traco(r, tf);
            };
            let sg = sigma_svg(6.0 + 10.0 * e2);
            if sg > 0.0 { com_sombra(mp, limites(&p, larg), 0.0, 0.0, sg, cor, &traco, &tudo) } else { tudo(mp, Transform::identity()) }
        }
        // magno: so um ponto claro no litoral enquanto a cidade ainda nao aparece
        if mg {
            let op = interp(z, &[0.0, 0.1, 0.55, 0.7], &[0.0, 1.0, 1.0, 0.0]);
            if let Some(c) = PathBuilder::from_circle(centro.0, centro.1, 7.0) {
                encher(mp, &c, Cor(0.55, 0.95, 1.0, op));
            }
        }
        // marcador pulsante no centro do municipio
        if let Some(c) = PathBuilder::from_circle(centro.0, centro.1, lv * 0.03 * pulso * s).filter(|_| !mg) {
            tracar(mp, &c, cor.a(marcador * 0.8), 4.0);
        }
        if let Some(c) = PathBuilder::from_circle(centro.0, centro.1, lv * 0.011 * s).filter(|_| !mg) {
            encher(mp, &c, Cor(1.0, 1.0, 1.0, marcador));
            tracar(mp, &c, cor, 5.0);
        }
        if mg {
            let nome = g.alvo_nome.to_uppercase();
            let w = largura(&fo.negrito, 34.0, &nome, 1.5);
            let (_, y0) = tela(cc.x0, cc.y0);
            let (_, y1) = tela(cc.x1, cc.y1);
            let op_nome = interp(z, &[0.55, 0.75], &[0.0, 1.0]) * interp(frame, &[dur - 4.0, dur], &[1.0, 0.0]); // no video o nome sai antes do corte
            let base = y0 + 0.3 * (y1 - y0);
            texto_contorno(mp, &fo.negrito, 34.0, centro.0 - w / 2.0 + 0.03 * l, base, &nome, 1.5, 5.0, op_nome);
            // setinha branca embaixo do nome, apontando para a cidade (como no video)
            let (sx, sy) = (centro.0 + 0.03 * l, base + 12.0);
            let mut pb = PathBuilder::new();
            pb.move_to(sx - 8.0, sy);
            pb.line_to(sx + 8.0, sy);
            pb.line_to(sx, sy + 11.0);
            pb.close();
            if let Some(seta) = pb.finish().filter(|_| op_nome > 0.0) {
                tracar(mp, &seta, Cor(0.0, 0.0, 0.0, op_nome * 0.6), 3.0);
                encher(mp, &seta, Cor(1.0, 1.0, 1.0, op_nome));
            }
        } else if cheia && e2 > 0.3 {
            let nome = g.alvo_nome.to_uppercase();
            let w = largura(&fo.mono, 46.0, &nome, 2.0);
            texto_contorno(mp, &fo.mono, 46.0, centro.0 - w / 2.0, centro.1 - lv * 0.045 * s, &nome, 2.0, 6.0, ((e2 - 0.3) * 3.0).min(1.0));
        }
        let pino_op = interp(e2, &[0.55, 1.0], &[0.0, 1.0]);
        for (i, pn) in dados["pinos"].as_array().into_iter().flatten().take(8).enumerate() {
            let (Some(lat), Some(lon)) = (pn["lat"].as_f64(), pn["lon"].as_f64()) else { continue };
            let pop = interp(frame - i as f32 * 6.0, &[0.0, 10.0], &[0.0, 1.0]) * pino_op;
            if pop <= 0.0 {
                continue;
            }
            let (x, y) = { let q = pj(lon, lat); tela(q.0, q.1) };
            if let Some(c) = PathBuilder::from_circle(x, y, 11.0) {
                encher(mp, &c, Cor::hex("#ef4444").a(pop));
                tracar(mp, &c, Cor(1.0, 1.0, 1.0, pop), 3.0);
            }
            let nome: String = pn["nome"].as_str().unwrap_or("").chars().take(30).collect();
            let w = largura(&fo.sans, 30.0, &nome, 0.0);
            let tx = if x / l > 0.55 { x - 18.0 - w } else { x + 18.0 };
            texto_contorno(mp, &fo.sans, 30.0, tx, y + 8.0, &nome, 0.0, 5.0, pop);
        }
    }

    if let Some(c) = carta {
        inclinar(&mut camada, c, cor, texto_cor, e1, e2, &g, fo);
    } else if mg {
        enfeites_magno(&mut camada, dados, frame / FPS, dur / FPS, fo);
    } else {
        enfeites(&mut camada, dados, frame / FPS, fo);
    }
    if let Some((_, credito)) = &g.sat {
        let (fs, base, op) = if cheia { (15.0, 24.0, 0.35) } else { (21.0, 70.0, 0.55) };
        let linhas = quebrar(&fo.sans, fs, credito, 940.0);
        let (lh, bl) = linha(&fo.sans, fs);
        let topo = 1920.0 - base - lh * linhas.len() as f32;
        for (i, ln) in linhas.iter().enumerate() {
            let w = largura(&fo.sans, fs, ln, 0.0);
            if let Some(p) = texto(&fo.sans, fs, 540.0 - w / 2.0, topo + i as f32 * lh + bl, ln, 0.0) {
                encher(&mut camada, &p, texto_cor.a(op));
            }
        }
    }
    let paint = PixmapPaint { opacity: entrada, ..Default::default() };
    pm.draw_pixmap(0, 0, camada.as_ref(), &paint, Transform::identity(), None);
}

/// visual "magno": cor do satelite ajustada ao video (saturacao 1,8 e ganho por canal, minimos quadrados em 594 blocos de 60 px
/// de 3 quadros; erro medio caiu de 24 para 18 em 255). O mosaico aberto e mais claro e lavado que o do video.
fn graduar(pm: &mut Pixmap, forca: f32) {
    const G: [(f32, f32); 3] = [(0.82, 0.006), (0.607, 0.033), (0.773, -0.028)];
    for p in pm.data_mut().chunks_exact_mut(4) {
        let c = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
        let l = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        for k in 0..3 {
            let v = (l + (c[k] - l) * 1.8) * G[k].0 + G[k].1;
            let v = c[k] + (v - c[k]) * forca;
            p[k] = (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }
    }
}

/// encolhe `v` (mesma proporcao, em volta do centro) ate caber em `limite` e o empurra para dentro dele
fn caber(v: Caixa, limite: Caixa) -> Caixa {
    let k = ((limite.x1 - limite.x0) / (v.x1 - v.x0)).min((limite.y1 - limite.y0) / (v.y1 - v.y0)).min(1.0);
    let (w, h) = ((v.x1 - v.x0) * k, (v.y1 - v.y0) * k);
    let cx = ((v.x0 + v.x1) / 2.0).clamp(limite.x0 + w / 2.0, limite.x1 - w / 2.0);
    let cy = ((v.y0 + v.y1) / 2.0).clamp(limite.y0 + h / 2.0, limite.y1 - h / 2.0);
    Caixa { x0: cx - w / 2.0, y0: cy - h / 2.0, x1: cx + w / 2.0, y1: cy + h / 2.0 }
}

/// visual "magno": voo continuo do Brasil ate a cidade ocupar ~28% da largura, centrada em x e um pouco abaixo do meio.
/// Largura da vista interpolada em escala log; o centro anda junto com o zoom (como um "flyTo"). Devolve a vista e o progresso z (0..1).
fn voo(v0: Caixa, cidade: Caixa, prop: f32, t: f32) -> (Caixa, f32) {
    let w1 = (cidade.x1 - cidade.x0) / 0.27;
    let h1 = w1 / prop;
    let c1 = ((cidade.x0 + cidade.x1) / 2.0, (cidade.y0 + cidade.y1) / 2.0 - 0.08 * h1);
    let w0 = v0.x1 - v0.x0;
    let c0 = ((v0.x0 + v0.x1) / 2.0, (v0.y0 + v0.y1) / 2.0);
    let z = suave(t, 0.6, 8.5);
    // depois do voo o mapa nunca para: segue aproximando bem devagar (~2,5% por segundo), como no video.
    // A velocidade entra aos poucos (rampa de 6 a 9 s) para nao ter tranco quando o voo principal desacelera
    let rampa = if t < 6.0 { 0.0 } else if t < 9.0 { (t - 6.0).powi(2) / 6.0 } else { 1.5 + (t - 9.0) };
    let w = w0 * (w1 / w0).powf(z) * (-0.025 * rampa).exp();
    // o centro chega no litoral da cidade antes de o zoom terminar (no video, aos ~3 s ja esta perto)
    let k = suave(t, 0.6, 5.0);
    let (cx, cy) = (c0.0 + (c1.0 - c0.0) * k, c0.1 + (c1.1 - c0.1) * k);
    let h = w / prop;
    (Caixa { x0: cx - w / 2.0, y0: cy - h / 2.0, x1: cx + w / 2.0, y1: cy + h / 2.0 }, z)
}

/// visual "magno": plaquinhas que se acumulam (digitadas letra a letra), apagadas quando entra a faixa;
/// faixa verde e etiqueta do TCM tambem digitadas. t em segundos da cena.
fn enfeites_magno(pm: &mut Pixmap, dados: &Value, t: f32, fim: f32, fo: &Fontes) {
    let tempo = |m: &Value| m["t"].as_f64().unwrap_or(0.0) as f32;
    let tem = |m: &Value| m["rotulo"].as_str().is_some_and(|r| !r.is_empty());
    let (etq, fx) = (&dados["etiqueta"], &dados["faixa"]);
    let t_faixa = if tem(fx) { tempo(fx) } else { f32::MAX };
    let t_etq = if tem(etq) { tempo(etq) } else { f32::MAX };
    // caixa no tamanho do texto inteiro; o texto aparece letra a letra
    let caixa = |pm: &mut Pixmap, txt: &str, cy: f32, fs: f32, fundo: Cor, linha_cor: Cor, frac: f32| {
        if frac <= 0.0 {
            return;
        }
        let tw = largura(&fo.preto, fs, txt, 0.0);
        let (bw, bh) = (tw + 28.0, fs * 1.25 + 4.0);
        let (x, y) = (540.0 - bw / 2.0, cy - bh / 2.0);
        ret(pm, x, y, bw, bh, 5.0, fundo);
        borda(pm, x, y, bw, bh, 5.0, linha_cor, 2.5);
        let n = txt.chars().count();
        let parte: String = txt.chars().take(((n as f32) * frac).ceil() as usize).collect();
        if let Some(p) = texto(&fo.preto, fs, x + 14.0, cy + 0.36 * fs, &parte, 0.0) {
            encher(pm, &p, Cor(1.0, 1.0, 1.0, 1.0));
        }
    };
    let escuro = Cor::rgba(14, 14, 16, 0.82);
    let branco = Cor(1.0, 1.0, 1.0, 0.95);
    let ini_some = t_faixa.min(t_etq);
    // as plaquinhas se apagam de tras para frente quando entra a faixa
    let some = interp(t, &[ini_some, ini_some + 0.35], &[1.0, 0.0]);
    for (i, p) in dados["plaquinhas"].as_array().into_iter().flatten().enumerate() {
        let (Some(r), t0) = (p["rotulo"].as_str(), tempo(p)) else { continue };
        // no video o ano vem antes do complemento: "R$ 10,89 BI · 2026 ATE SET"
        let (valor, resto) = r.split_once(" · ").map(|(a, b)| (a, format!(" {b}"))).unwrap_or((r, String::new()));
        let ano = p["ano"].as_str().map(|a| format!(" · {a}{resto}")).unwrap_or(resto);
        let txt = if i == 0 { format!("ARRECADOU {valor}{ano}") } else { format!("{valor}{ano}") }.to_uppercase();
        let frac = interp(t, &[t0, t0 + 0.6], &[0.0, 1.0]).min(some);
        caixa(pm, &txt, 638.0 + i as f32 * 80.0, 34.0, escuro, branco, frac);
    }
    let visivel = t < fim - 0.12; // no video faixa e etiqueta saem um pouco antes do fim da cena
    if tem(etq) && visivel {
        let frac = interp(t, &[t_etq, t_etq + 0.5], &[0.0, 1.0]);
        caixa(pm, &etq["rotulo"].as_str().unwrap_or("").to_uppercase(), 677.0, 34.0, escuro, branco, frac);
    }
    if tem(fx) && visivel {
        let frac = interp(t, &[t_faixa + 0.15, t_faixa + 0.65], &[0.0, 1.0]);
        caixa(pm, &fx["rotulo"].as_str().unwrap_or("").to_uppercase(), 1380.0, 24.0, Cor::rgba(47, 163, 107, 0.95), Cor(0.85, 1.0, 0.9, 0.9), frac);
    }
}

/// visual "magno": sobreposicoes do mapa prolongado depois do fim da cena (como no video, ~15 a 18,4 s):
/// etiqueta do ano digitada, depois apagada; "ARRECADOU EM <ano>" digitado e o valor grande com brilho verde.
/// tl = segundos desde o fim da cena do mapa; ta = instante do "arrecadou" (mesma escala).
pub fn fim_magno(pm: &mut Pixmap, ano: &str, valor: &str, tl: f32, ta: f32) {
    let fo = fontes();
    let cx = 660.0; // no video o bloco fica a direita do centro
    let caixa = |pm: &mut Pixmap, txt: &str, cy: f32, fs: f32, frac: f32| {
        if frac <= 0.0 {
            return;
        }
        let tw = largura(&fo.preto, fs, txt, 0.0);
        let (bw, bh) = (tw + 28.0, fs * 1.25 + 4.0);
        let (x, y) = (cx - bw / 2.0, cy - bh / 2.0);
        ret(pm, x, y, bw, bh, 7.0, Cor::rgba(14, 14, 16, 0.85));
        borda(pm, x, y, bw, bh, 7.0, Cor(1.0, 1.0, 1.0, 0.95), 2.5);
        let n = txt.chars().count();
        let parte: String = txt.chars().take(((n as f32) * frac).ceil() as usize).collect();
        if let Some(p) = texto(&fo.preto, fs, x + 14.0, cy + 0.36 * fs, &parte, 0.0) {
            encher(pm, &p, Cor(1.0, 1.0, 1.0, 1.0));
        }
    };
    // ano digitado no comeco e apagado (de tras para frente) logo antes do valor
    let f_ano = interp(tl, &[0.0, 0.15], &[0.0, 1.0]).min(interp(tl, &[ta - 0.45, ta - 0.2], &[1.0, 0.0]));
    caixa(pm, ano, 598.0, 40.0, f_ano);
    caixa(pm, &format!("ARRECADOU EM {ano}"), 699.0, 34.0, interp(tl, &[ta - 0.45, ta - 0.05], &[0.0, 1.0]));
    let frac = interp(tl, &[ta - 0.1, ta + 0.25], &[0.0, 1.0]);
    if frac <= 0.0 {
        return;
    }
    // "R$ 12,20 BI": a unidade final sai menor, como no video
    let (num, uni) = valor.rsplit_once(' ').filter(|(_, u)| u.chars().all(char::is_alphabetic)).unwrap_or((valor, ""));
    let (fn_, fu) = (96.0, 42.0);
    let (wn, wu) = (largura(&fo.preto, fn_, num, 0.0), if uni.is_empty() { 0.0 } else { 30.0 + largura(&fo.preto, fu, uni, 0.0) });
    let x = cx - (wn + wu) / 2.0;
    let base = 640.0;
    let total = num.chars().count() + uni.chars().count();
    let vis = ((total as f32) * frac).ceil() as usize;
    let pn: String = num.chars().take(vis).collect();
    let pu: String = uni.chars().take(vis.saturating_sub(num.chars().count())).collect();
    let mut pb = PathBuilder::new();
    for c in [texto(&fo.preto, fn_, x, base, &pn, 0.0), texto(&fo.preto, fu, x + wn + 30.0, base, &pu, 0.0)].into_iter().flatten() {
        pb.push_path(&c);
    }
    let Some(p) = pb.finish() else { return };
    let pinta = |r: &mut Pixmap, tf: Transform| r.fill_path(&p, &Cor(1.0, 1.0, 1.0, 1.0).paint(), FillRule::Winding, tf, None);
    com_sombra(pm, limites_de(&p), 0.0, 0.0, 9.0, Cor::hex("#34d399").a(0.8), &pinta, &pinta);
}
fn limites_de(p: &Caminho) -> (f32, f32, f32, f32) {
    let b = p.bounds();
    (b.left(), b.top(), b.right(), b.bottom())
}

fn quebrar(f: &FontVec, px: f32, txt: &str, max: f32) -> Vec<String> {
    let mut linhas = Vec::new();
    let mut atual = String::new();
    for w in txt.split_whitespace() {
        let t = if atual.is_empty() { w.to_string() } else { format!("{atual} {w}") };
        if !atual.is_empty() && largura(f, px, &t, 0.0) > max {
            linhas.push(std::mem::replace(&mut atual, w.to_string()));
        } else {
            atual = t;
        }
    }
    if !atual.is_empty() {
        linhas.push(atual);
    }
    linhas
}

// pilula, etiqueta do TCM e faixa verde sobre o mapa de tela cheia (Enfeites do Mapa.tsx); t em segundos da cena
fn enfeites(pm: &mut Pixmap, dados: &Value, t: f32, fo: &Fontes) {
    let ve = |m: &Value| m["rotulo"].as_str().is_some_and(|r| !r.is_empty()) && t >= m["t"].as_f64().unwrap_or(0.0) as f32;
    let plaq = dados["plaquinhas"].as_array().into_iter().flatten().filter(|p| p["t"].as_f64().unwrap_or(0.0) as f32 <= t).last();
    let pilula = |pm: &mut Pixmap, txt: &str, top: f32, fs: f32, fundo: Cor, com_borda: bool| {
        let txt = txt.to_uppercase();
        let b = if com_borda { 3.0 } else { 0.0 };
        let (lh, bl) = linha(&fo.mono, fs);
        let tw = largura(&fo.mono, fs, &txt, 2.0);
        let (bw, bh) = (tw + 48.0 + 2.0 * b, lh + 20.0 + 2.0 * b);
        let x = 540.0 - bw / 2.0;
        ret(pm, x, top, bw, bh, 12.0, fundo);
        if com_borda {
            borda(pm, x, top, bw, bh, 12.0, Cor(1.0, 1.0, 1.0, 1.0), b);
        }
        if let Some(p) = texto(&fo.mono, fs, x + b + 24.0, top + b + 10.0 + bl, &txt, 2.0) {
            encher(pm, &p, Cor(1.0, 1.0, 1.0, 1.0));
        }
    };
    let escuro = Cor::rgba(12, 12, 14, 0.85);
    let (etq, fx) = (&dados["etiqueta"], &dados["faixa"]);
    if ve(etq) {
        pilula(pm, etq["rotulo"].as_str().unwrap_or(""), 660.0, 30.0, escuro, true);
    } else if let Some(r) = plaq.and_then(|p| p["rotulo"].as_str()) {
        let ano = plaq.and_then(|p| p["ano"].as_str()).map(|a| format!(" · {a}")).unwrap_or_default();
        pilula(pm, &format!("ARRECADOU {r}{ano}"), 640.0, 32.0, escuro, true);
    }
    if ve(fx) {
        pilula(pm, fx["rotulo"].as_str().unwrap_or(""), 1290.0, 30.0, Cor::rgba(46, 160, 90, 0.95), false);
    }
}

// carta inclinada: rotateX(22deg) com origem em 50% 80%, perspectiva 1800 no pai (960x900 em x=60, y=385), cantos de 18 px e sombra
fn inclinar(camada: &mut Pixmap, mut carta: Pixmap, cor: Cor, texto_cor: Cor, e1: f32, e2: f32, g: &Geo, fo: &Fontes) {
    let (cw, ch) = (960usize, 900usize);
    // cantos arredondados (overflow hidden + border-radius 18)
    let r = 18.0f32;
    let dc = carta.data_mut();
    for y in 0..ch {
        for x in 0..cw {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let (qx, qy) = ((r - fx).max(fx - (cw as f32 - r)), (r - fy).max(fy - (ch as f32 - r)));
            if qx > 0.0 && qy > 0.0 {
                let cob = (r - (qx * qx + qy * qy).sqrt() + 0.5).clamp(0.0, 1.0);
                let i = (y * cw + x) * 4;
                for k in 0..4 {
                    dc[i + k] = (dc[i + k] as f32 * cob) as u8;
                }
            }
        }
    }
    let (d, sn, cs) = (1800.0f32, 22f32.to_radians().sin(), 22f32.to_radians().cos());
    let (ox, oy) = (60.0f32, 385.0f32);
    let frente = |x: f32, y: f32| {
        let (u, w) = (x - 480.0, y - 720.0);
        let k = d / (d - w * sn);
        (ox + 480.0 + u * k, oy + 450.0 + (270.0 + w * cs) * k)
    };
    let cantos = [frente(0.0, 0.0), frente(960.0, 0.0), frente(0.0, 900.0), frente(960.0, 900.0)];
    let bx0 = cantos.iter().map(|c| c.0).fold(f32::MAX, f32::min).floor().max(0.0) as i32;
    let bx1 = cantos.iter().map(|c| c.0).fold(f32::MIN, f32::max).ceil().min(1080.0) as i32;
    let by0 = cantos.iter().map(|c| c.1).fold(f32::MAX, f32::min).floor().max(0.0) as i32;
    let by1 = cantos.iter().map(|c| c.1).fold(f32::MIN, f32::max).ceil().min(1920.0) as i32;
    let Some(mut proj) = Pixmap::new(1080, 1920) else { return };
    {
        let src = carta.data();
        let dp = proj.data_mut();
        for py in by0..by1 {
            let yy = py as f32 + 0.5 - oy - 450.0;
            let w = d * (yy - 270.0) / (yy * sn + cs * d);
            let k = d / (d - w * sn);
            let sy = w + 720.0 - 0.5;
            if !(-1.0..ch as f32).contains(&sy) {
                continue;
            }
            for px in bx0..bx1 {
                let sx = (px as f32 + 0.5 - ox - 480.0) / k + 480.0 - 0.5;
                if !(-1.0..cw as f32).contains(&sx) {
                    continue;
                }
                // bilinear com borda transparente
                let (x0, y0) = (sx.floor() as i32, sy.floor() as i32);
                let (fx, fy) = (sx - x0 as f32, sy - y0 as f32);
                let mut acc = [0.0f32; 4];
                for (dx, dy, wgt) in [(0, 0, (1.0 - fx) * (1.0 - fy)), (1, 0, fx * (1.0 - fy)), (0, 1, (1.0 - fx) * fy), (1, 1, fx * fy)] {
                    let (qx, qy) = (x0 + dx, y0 + dy);
                    if qx >= 0 && qy >= 0 && (qx as usize) < cw && (qy as usize) < ch {
                        let i = (qy as usize * cw + qx as usize) * 4;
                        for k2 in 0..4 {
                            acc[k2] += src[i + k2] as f32 * wgt;
                        }
                    }
                }
                let i = ((py * 1080 + px) * 4) as usize;
                for k2 in 0..4 {
                    dp[i + k2] = (acc[k2] + 0.5) as u8;
                }
            }
        }
    }
    // drop-shadow(0 38px 30px rgba(0,0,0,.55)) da carta ja inclinada
    let area = (bx0 as f32, by0 as f32, bx1 as f32, by1 as f32);
    let carta_pinta = |r: &mut Pixmap, tf: Transform| r.draw_pixmap(0, 0, proj.as_ref(), &PixmapPaint::default(), tf, None);
    com_sombra(camada, area, 0.0, 38.0, 15.0, Cor(0.0, 0.0, 0.0, 0.55), &carta_pinta, &carta_pinta);
    // rotulo do nivel embaixo da carta
    let nivel = if e2 > 0.5 { g.alvo_nome.clone() } else if e1 > 0.5 { g.nome_uf.clone() } else { "Brasil".into() };
    let topo = if e2 > 0.5 { format!("{} · município", g.nome_uf) } else if e1 > 0.5 { "Estado".into() } else { "País".into() };
    let topo = topo.to_uppercase();
    let (lh1, bl1) = linha(&fo.sans, 30.0);
    let w1 = largura(&fo.sans, 30.0, &topo, 6.0);
    if let Some(p) = texto(&fo.sans, 30.0, 540.0 - w1 / 2.0, 1300.0 + bl1, &topo, 6.0) {
        encher(camada, &p, texto_cor.a(0.7));
    }
    let lh2 = 68.0 * 1.1;
    let bl2 = { let (l0, b0) = linha(&fo.sans, 68.0); b0 + (lh2 - l0) / 2.0 };
    let w2 = largura(&fo.sans, 68.0, &nivel, 0.0);
    if let Some(p) = texto(&fo.sans, 68.0, 540.0 - w2 / 2.0, 1300.0 + lh1 + bl2, &nivel, 0.0) {
        let b = p.bounds();
        let pinta = |r: &mut Pixmap, tf: Transform| r.fill_path(&p, &texto_cor.paint(), FillRule::Winding, tf, None);
        com_sombra(camada, (b.left(), b.top(), b.right(), b.bottom()), 0.0, 4.0, 12.0, cor.a(0x88 as f32 / 255.0), &pinta, &pinta);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn slug_igual_ao_remotion() {
        assert_eq!(slug("Santa Cruz Cabrália"), "santa-cruz-cabralia");
        assert_eq!(slug("Salvador"), "salvador");
        assert_eq!(slug("  São João d'Aliança "), "sao-joao-d-alianca");
        assert_eq!(slug(""), "x");
    }
    #[test]
    fn mercator_e_enquadramento() {
        let (x, y) = pj(0.0, 0.0);
        assert!((x - 500.0).abs() < 1e-3 && (y - 500.0).abs() < 1e-3);
        let c = enquadrar(Caixa { x0: 0.0, y0: 0.0, x1: 10.0, y1: 10.0 }, 0.5, 0.0);
        assert!(((c.x1 - c.x0) / (c.y1 - c.y0) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn voo_magno_comeca_no_inicio_e_termina_na_cidade() {
        let (v0, cidade, prop) = (Caixa { x0: 0.0, y0: 0.0, x1: 100.0, y1: 200.0 }, Caixa { x0: 60.0, y0: 120.0, x1: 62.7, y1: 122.0 }, 0.5);
        let (a, za) = voo(v0, cidade, prop, 0.0);
        assert!(za == 0.0 && (a.x1 - a.x0 - 100.0).abs() < 1e-3);
        let (b, zb) = voo(v0, cidade, prop, 9.0);
        assert!(zb == 1.0 && ((cidade.x1 - cidade.x0) / (b.x1 - b.x0) - 0.27 * (0.025f32 * 1.5).exp()).abs() < 1e-3);
        let (c, _) = voo(v0, cidade, prop, 14.0);
        assert!(c.x1 - c.x0 < b.x1 - b.x0); // continua aproximando depois do voo
        assert!(b.x0 < 60.0 && b.x1 > 62.7 && b.y0 < 120.0 && b.y1 > 122.0);
        let d = caber(Caixa { x0: -10.0, y0: -10.0, x1: 30.0, y1: 70.0 }, Caixa { x0: 0.0, y0: 0.0, x1: 40.0, y1: 40.0 });
        assert!(d.x0 >= 0.0 && d.y0 >= 0.0 && d.x1 <= 40.0 && d.y1 <= 40.0 && ((d.x1 - d.x0) / (d.y1 - d.y0) - 0.5).abs() < 1e-4);
    }
}
