// Monta um quadro do video a partir do roteiro, na mesma ordem e medidas do src/Estudio.tsx.
use crate::desenho::{borda, br, circulo, interp, mola, ret, Cor, Fonte};
use crate::roteiro::{estilo, juntar_numeros, Cabecalho, Cena, Estilo, Roteiro, ALT, FPS, LARG};
use serde_json::Value;
use std::path::Path;
use tiny_skia::{
    FillRule, FilterQuality, GradientStop, LinearGradient, LineCap, LineJoin, Paint, PathBuilder, Pixmap, PixmapPaint,
    Point, SpreadMode, Stroke, Transform,
};

const BRANCO_MESA: &str = "#F0EBE1";
/// magno: pilula do titulo do cabecalho (x, y, altura), medida no video
const MG_CAB: (f32, f32, f32) = (60.0, 330.0, 68.0);

pub struct Motor {
    pub r: Roteiro,
    e: Estilo,
    reg: Fonte,
    neg: Fonte,
    mono: Fonte,
    arial: Fonte,
    arial_neg: Fonte,
    fundo: Pixmap,
    selo_g: Option<Pixmap>,
    selo_p: Option<Pixmap>,
    selo_m: Option<Pixmap>,
    pub total: u32,
}

impl Motor {
    pub fn novo(r: Roteiro, raiz: &Path) -> Result<Motor, String> {
        let e = estilo(&r.estilo);
        let foto = image::open(raiz.join("public/marca/eumagnoleite.jpg")).ok().map(|i| i.to_rgba8());
        let total = r.quadros();
        Ok(Motor {
            reg: Fonte::abrir(e.fontes.0)?,
            neg: Fonte::abrir(e.fontes.1)?,
            mono: Fonte::abrir("consolab.ttf")?,
            arial: Fonte::abrir("arial.ttf")?,
            arial_neg: Fonte::abrir("arialbd.ttf")?,
            // magno: fundo quase preto e quente, medido no video (#110908 em cima, #0c0506 embaixo)
            fundo: if r.magno() { fundo(&Estilo { fundo_a: Cor::hex("#140b09"), fundo_b: Cor::hex("#0b0505"), ..estilo(&r.estilo) }) } else { fundo(&e) },
            selo_g: foto.as_ref().map(|f| selo(f, 140)),
            selo_p: foto.as_ref().map(|f| selo(f, 56)),
            selo_m: foto.as_ref().filter(|_| r.magno()).map(|f| selo(f, 120)),
            total,
            e,
            r,
        })
    }

    pub fn quadro(&self, f: u32) -> Pixmap {
        let mut pm = self.fundo.clone();
        let t = f as f64 / FPS;
        for (i, c) in self.r.cenas.iter().enumerate() {
            let de = (c.inicio * FPS).round() as i64;
            let dur = ((c.fim * FPS).round() as i64 - de).max(1);
            let lf = f as i64 - de;
            if (0..dur).contains(&lf) {
                let alem = self.mapa_alem(i, t as f32);
                if alem.map_or(true, |(_, op)| op < 1.0) {
                    self.cena(&mut pm, c, lf as f32, dur as f32);
                }
                if let Some((m, op)) = alem {
                    self.mapa_prolongado(&mut pm, m, c, t as f32, op);
                }
            }
        }
        if let Some(cab) = &self.r.cabecalho {
            self.cabecalho(&mut pm, cab, t as f32);
            if let Some(a) = &cab.arroba {
                self.marca_dagua(&mut pm, a, t as f32);
                if self.r.magno() {
                    self.selo_magno(&mut pm, a, t as f32);
                } else {
                    self.selo_grande(&mut pm, a, t as f32);
                }
            }
        }
        self.legenda(&mut pm, t);
        pm
    }

    fn cena(&self, pm: &mut Pixmap, c: &Cena, lf: f32, dur: f32) {
        match c.peca.as_str() {
            "numero" => self.numero(pm, &c.dados, lf),
            "linha" => self.linha(pm, &c.dados, lf, dur),
            "mapa" => crate::mapa::desenhar(pm, &c.dados, lf, dur, crate::mapa::raiz(), self.e.destaque, self.e.texto, self.r.magno(), 0.0),
            "maquete" => {
                // magno: visao geral logo depois da faixa do comparativo: a faixa continua na mesa (sem "teleporte")
                let ant = self.r.cenas.iter().position(|x| std::ptr::eq(x, c)).and_then(|i| i.checked_sub(1)).map(|i| &self.r.cenas[i]);
                let mut dados = std::borrow::Cow::Borrowed(&c.dados);
                if self.r.magno() {
                    // valores de todos os setores do mesmo ano: o anel do cofre ja nasce separado por setor
                    let mut m = serde_json::Map::new();
                    for x in self.r.cenas.iter().filter(|x| x.peca == "maquete" && x.dados["ano"] == c.dados["ano"]) {
                        for st in x.dados["setores"].as_array().into_iter().flatten() {
                            if let (Some(n), Some(v)) = (st["nome"].as_str(), st["valor"].as_f64()) {
                                m.entry(n.to_string()).or_insert(Value::from(v));
                            }
                        }
                    }
                    dados.to_mut()["valores_ano"] = Value::Object(m);
                }
                if let Some(a) = ant.filter(|a| self.r.magno() && e_comparativo(a) && c.dados["visao_geral"].as_bool().unwrap_or(false)) {
                    let d = dados.to_mut();
                    d["faixa_ant"] = comparativo_acumulado(&self.r.cenas, a);
                    d["faixa_ant_frames"] = Value::from((a.fim - a.inicio) * FPS as f64);
                }
                crate::tres::desenhar(&crate::maquete::cena(&dados, lf, dur, false, self.r.magno()), pm, superamostra());
                self.etiqueta_maquete(pm, &c.dados, lf / FPS as f32, dur / FPS as f32);
            }
            "barras" if c.dados["mesa"].as_bool().unwrap_or(false) && self.r.magno() => {
                let mut d = comparativo_acumulado(&self.r.cenas, c);
                // 1a cena da faixa (a anterior nao e comparativo): a camera chega deslizando e a faixa cresce no lugar
                let i = self.r.cenas.iter().position(|x| std::ptr::eq(x, c)).unwrap_or(0);
                d["faixa_primeira"] = Value::from(i == 0 || !e_comparativo(&self.r.cenas[i - 1]));
                crate::tres::desenhar(&crate::maquete::cena(&d, lf, dur, true, true), pm, superamostra());
                pilula_comparativo(pm, c, lf / FPS as f32);
            }
            "barras" if c.dados["mesa"].as_bool().unwrap_or(false) => crate::tres::desenhar(&crate::maquete::cena(&c.dados, lf, dur, true, self.r.magno()), pm, superamostra()),
            _ => {} // peca 3D ainda nao portada (barras sem mesa, globo, pilha, moedas): fica so o fundo
        }
        if let (Some(tit), None) = (&c.titulo, &self.r.cabecalho) {
            self.titulo_cena(pm, tit, lf);
        }
    }

    // ---------- pecas ----------

    fn numero(&self, pm: &mut Pixmap, d: &Value, lf: f32) {
        let ent = mola(lf, 30.0, 14.0);
        let casas = d["casas"].as_u64().unwrap_or(0) as usize;
        let conta = interp(lf, &[4.0, 40.0], &[0.0, 1.0]) as f64;
        let exib = match &d["valor"] {
            Value::Number(n) => br(n.as_f64().unwrap_or(0.0) * conta, casas),
            Value::Null => String::new(),
            Value::String(s) => s.clone(),
            v => v.to_string(),
        };
        let grande = format!("{}{}{}", txt(d, "prefixo"), exib, txt(d, "sufixo"));
        let n = grande.chars().count();
        let tam = if n > 12 { 118.0 } else if n > 8 { 150.0 } else { 190.0 };
        let brilho = Some((self.e.destaque.a(0x77 as f32 / 255.0), 60.0)); // textShadow 0 0 60px cor77
        let mut itens = vec![Item { f: &self.reg, px: tam, lh: Some(tam), cor: self.e.destaque, mt: 0.0, txt: grande, brilho }];
        if let Some(r) = d["rotulo"].as_str() {
            itens.push(Item { f: &self.reg, px: 58.0, lh: None, cor: self.e.texto.a(0.95), mt: 34.0, txt: r.into(), brilho: None });
        }
        if let Some(r) = d["detalhe"].as_str() {
            itens.push(Item { f: &self.reg, px: 40.0, lh: None, cor: self.e.texto.a(0.7), mt: 20.0, txt: r.into(), brilho: None });
        }
        let altura = coluna_altura(&itens, 920.0);
        let topo = ALT as f32 / 2.0 - altura / 2.0;
        coluna(pm, &itens, 920.0, topo, 0.6 + 0.4 * ent, ent);
    }

    fn linha(&self, pm: &mut Pixmap, d: &Value, lf: f32, dur: f32) {
        let mut valores: Vec<f32> = d["valores"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).map(|v| v as f32).take(48).collect()).unwrap_or_default();
        if valores.is_empty() {
            valores = vec![1.0, 3.0, 2.0, 5.0];
        }
        if valores.len() == 1 {
            valores.push(valores[0]);
        }
        let rotulos: Vec<String> = d["rotulos"].as_array().map(|a| a.iter().map(|v| v.as_str().map(String::from).unwrap_or(v.to_string())).collect()).unwrap_or_default();
        let casas = d["casas"].as_u64().unwrap_or(0) as usize;
        let ultimo = if *valores.last().unwrap() == 0.0 { 1.0 } else { *valores.last().unwrap() };
        let fator = d["final"].as_f64().map(|f| f as f32 / ultimo).unwrap_or(1.0);
        let (l, a, x0, y0) = (900.0f32, 620.0f32, 60.0f32, 40.0f32);
        let min = valores.iter().cloned().fold(0.0, f32::min);
        let max = valores.iter().cloned().fold(1e-9, f32::max);
        let nv = valores.len();
        let pontos: Vec<(f32, f32)> = valores
            .iter()
            .enumerate()
            .map(|(i, v)| (x0 + (i as f32 / (nv - 1) as f32) * (l - x0 * 1.2), y0 + (1.0 - (v - min) / (max - min)) * (a - y0 * 2.0)))
            .collect();
        let tl = interp(lf, &[8.0, (dur - 20.0).max(40.0)], &[0.0, 1.0]);
        let t = if tl < 0.5 { 4.0 * tl * tl * tl } else { 1.0 - (-2.0 * tl + 2.0).powi(3) / 2.0 };
        let pos = t * (nv - 1) as f32;
        let i0 = (pos.floor() as usize).min(nv - 2);
        let fr = pos - i0 as f32;
        let ponta = (pontos[i0].0 + (pontos[i0 + 1].0 - pontos[i0].0) * fr, pontos[i0].1 + (pontos[i0 + 1].1 - pontos[i0].1) * fr);
        let valor_ponta = valores[i0] + (valores[i0 + 1] - valores[i0]) * fr;
        let grande = format!("{}{}{}", txt(d, "prefixo"), br((valor_ponta * fator) as f64, casas), txt(d, "sufixo"));
        let n = grande.chars().count();
        let tam = if n > 12 { 104.0 } else if n > 9 { 130.0 } else { 170.0 };
        let ent = interp(lf, &[0.0, 12.0], &[0.0, 1.0]);

        let mut y = 400.0;
        let cx = LARG as f32 / 2.0;
        if let Some(r) = d["rotulo"].as_str() {
            let lh = self.reg.metricas(44.0).2;
            let w = self.reg.largura(44.0, r, 2.0);
            self.reg.desenhar(pm, 44.0, cx - w / 2.0, self.reg.baseline(44.0, y, lh), r, 2.0, self.e.texto.a(0.75 * ent));
            y += lh;
        }
        let lh = tam * 1.05;
        let w = self.reg.largura(tam, &grande, 0.0);
        let base = self.reg.baseline(tam, y, lh);
        self.reg.brilho(pm, tam, cx - w / 2.0, base, &grande, 0.0, self.e.destaque.a(0x66 as f32 / 255.0 * ent), 50.0);
        self.reg.desenhar(pm, tam, cx - w / 2.0, base, &grande, 0.0, self.e.destaque.a(ent));
        y += lh + 30.0;

        let (ox, oy) = (cx - l / 2.0, y);
        let cor = self.e.destaque;
        for g in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let gy = oy + y0 + g * (a - y0 * 2.0);
            let mut pb = PathBuilder::new();
            pb.move_to(ox + x0, gy);
            pb.line_to(ox + l - x0 * 0.2, gy);
            if let Some(p) = pb.finish() {
                let st = Stroke { width: 2.0, ..Default::default() };
                pm.stroke_path(&p, &Cor(1.0, 1.0, 1.0, if g == 1.0 { 0.45 } else { 0.12 } * ent).paint(), &st, Transform::identity(), None);
            }
        }
        let parte: Vec<(f32, f32)> = pontos[..=i0].iter().cloned().chain(std::iter::once(ponta)).collect();
        let mut area = PathBuilder::new();
        let mut traco = PathBuilder::new();
        for (i, (px, py)) in parte.iter().enumerate() {
            if i == 0 {
                area.move_to(ox + px, oy + py);
                traco.move_to(ox + px, oy + py);
            } else {
                area.line_to(ox + px, oy + py);
                traco.line_to(ox + px, oy + py);
            }
        }
        area.line_to(ox + ponta.0, oy + a - y0);
        area.line_to(ox + pontos[0].0, oy + a - y0);
        area.close();
        if let Some(p) = area.finish() {
            pm.fill_path(&p, &cor.a(0.12 * ent).paint(), FillRule::Winding, Transform::identity(), None);
        }
        if let Some(p) = traco.finish() {
            let st = Stroke { width: 9.0, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
            pm.stroke_path(&p, &cor.a(ent).paint(), &st, Transform::identity(), None);
        }
        circulo(pm, ox + ponta.0, oy + ponta.1, 20.0 + 3.0 * (lf * 0.3).sin(), cor.a(0.3 * ent));
        circulo(pm, ox + ponta.0, oy + ponta.1, 11.0, Cor(1.0, 1.0, 1.0, ent));
        let passo = if rotulos.len() > 12 { 2 } else { 1 };
        for (i, r) in rotulos.iter().enumerate() {
            if i % passo == 0 && i < pontos.len() {
                let w = self.reg.largura(28.0, r, 0.0);
                self.reg.desenhar(pm, 28.0, ox + pontos[i].0 - w / 2.0, oy + a + 24.0, r, 0.0, self.e.texto.a(0.7 * ent));
            }
        }
    }

    fn titulo_cena(&self, pm: &mut Pixmap, texto: &str, lf: f32) {
        let s = mola(lf, 30.0, 15.0);
        let px = 82.0;
        let lh = px * 1.08;
        let linhas = self.reg.quebrar(px, texto, 0.0, 860.0);
        let w = linhas.iter().map(|l| self.reg.largura(px, l, 0.0)).fold(0.0, f32::max);
        let topo = 170.0 + 40.0 * (1.0 - s);
        let cx = LARG as f32 / 2.0;
        for (i, l) in linhas.iter().enumerate() {
            let lw = self.reg.largura(px, l, 0.0);
            self.reg.desenhar(pm, px, cx - lw / 2.0, self.reg.baseline(px, topo + i as f32 * lh, lh), l, 0.0, self.e.texto.a(s));
        }
        ret(pm, cx - w / 2.0, topo + linhas.len() as f32 * lh + 18.0, w, 10.0, 0.0, self.e.destaque.a(s));
    }

    // ---------- camadas fixas (padrao Magno) ----------

    fn cabecalho(&self, pm: &mut Pixmap, cab: &Cabecalho, t: f32) {
        if self.r.magno() {
            return self.cabecalho_magno(pm, cab, t);
        }
        let (fundo, linha, claro) = (Cor::hex("#222127"), Cor::hex("#3A3840"), Cor::hex(BRANCO_MESA));
        let (x, y, h) = (90.0, 300.0, 74.0);
        let tit = cab.titulo.to_uppercase();
        let tw = self.mono.largura(31.0, &tit, 3.0);
        let w = 2.0 + 14.0 + 40.0 + 14.0 + tw + 26.0 + 2.0;
        ret(pm, x, y, w, h, 35.0, fundo);
        borda(pm, x, y, w, h, 35.0, linha, 2.0);
        let cy = y + h / 2.0;
        circulo(pm, x + 2.0 + 14.0 + 20.0, cy, 20.0, Cor::hex("#EDE8DE"));
        circulo(pm, x + 2.0 + 14.0 + 20.0, cy, 6.0, Cor::hex("#2FA36B"));
        let lh = self.mono.metricas(31.0).2;
        self.mono.desenhar(pm, 31.0, x + 2.0 + 14.0 + 40.0 + 14.0, self.mono.baseline(31.0, cy - lh / 2.0, lh), &tit, 3.0, claro);
        if let Some(a) = &cab.arroba {
            let sx = x + w + 16.0;
            if let Some(s) = &self.selo_p {
                pm.draw_pixmap(sx as i32, (cy - 28.0) as i32, s.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
            }
            let lh = self.mono.metricas(26.0).2;
            self.mono.desenhar(pm, 26.0, sx + 56.0 + 12.0, self.mono.baseline(26.0, cy - lh / 2.0, lh), a, 0.0, claro);
        }
        let mut cy0 = y + h;
        for (texto, fs, mt) in [
            (cab.fonte.as_deref(), cab.fonte.as_deref().map(|f| { let n = f.chars().count(); if n > 52 { 14.0 } else if n > 42 { 16.0 } else { 19.0 } }).unwrap_or(19.0), 14.0),
            (cab.escala.as_deref(), 16.0, 10.0),
        ] {
            let Some(texto) = texto else { continue };
            let t = texto.to_uppercase();
            let lh = self.mono.metricas(fs).2;
            let cw = 4.0 + 36.0 + self.mono.largura(fs, &t, 1.3);
            let ch = 4.0 + 16.0 + lh;
            let top = cy0 + mt;
            ret(pm, x, top, cw, ch, 11.0, fundo);
            borda(pm, x, top, cw, ch, 11.0, linha, 2.0);
            self.mono.desenhar(pm, fs, x + 2.0 + 18.0, self.mono.baseline(fs, top + 10.0, lh), &t, 1.3, claro);
            cy0 = top + ch;
        }
    }

    // magno (medido no video): pilula do titulo em x 60..378, y 330..398, circulo branco liso e ponto verde separado;
    // o @ vem do selo (selo_magno); os cartoes FONTE/escala entram em ~4,6 s, colados embaixo e recuados 30 px
    fn cabecalho_magno(&self, pm: &mut Pixmap, cab: &Cabecalho, t: f32) {
        let (fundo, linha, claro) = (Cor::hex("#222127"), Cor::hex("#3A3840"), Cor::hex(BRANCO_MESA));
        let (x, y, h) = MG_CAB;
        let w = self.cab_magno_larg(cab);
        let tit = cab.titulo.to_uppercase();
        ret(pm, x, y, w, h, h / 2.0, fundo);
        borda(pm, x, y, w, h, h / 2.0, linha, 2.0);
        let cy = y + h / 2.0;
        circulo(pm, x + 38.0, cy, 20.0, Cor::hex("#EDE8DE"));
        circulo(pm, x + 81.0, cy, 4.5, Cor::hex("#2FA36B"));
        let lh = self.mono.metricas(35.0).2;
        self.mono.desenhar(pm, 35.0, x + 105.0, self.mono.baseline(35.0, cy - lh / 2.0, lh), &tit, 3.4, claro);
        let op = interp(t, &[4.55, 4.75], &[0.0, 1.0]);
        if op <= 0.0 {
            return;
        }
        let mut top = y + h + 2.0;
        // legenda das tres moedas no lugar de "1 PILHA = ..."; na faixa do comparativo a escala e outra e a legenda avisa
        let resto = cab.escala.as_deref().map(|e| e.split_once(" · ").map(|x| format!(" · {}", x.1)).unwrap_or_default()).unwrap_or_default();
        let cena = self.r.cenas.iter().find(|c| (c.inicio as f32..c.fim as f32).contains(&t));
        let moedas = match cena {
            Some(c) if e_comparativo(c) => {
                let d = comparativo_acumulado(&self.r.cenas, c);
                let maior = d["max_faixa"].as_f64().unwrap_or(1.0) * d["faixa_unidade"].as_f64().unwrap_or(1.0);
                Some(format!("AQUI, {}", crate::maquete_magno::legenda_moedas(crate::maquete_magno::pilha_faixa_mi(maior))))
            }
            _ => self.r.cenas.iter().find_map(|c| c.dados["pilha_mi"].as_f64()).filter(|_| cab.escala.as_deref().is_some_and(|e| e.to_uppercase().starts_with("1 PILHA"))).map(|p| format!("{}{resto}", crate::maquete_magno::legenda_moedas(p))),
        };
        for (texto, fs) in [(cab.fonte.as_deref(), 17.0), (moedas.as_deref().or(cab.escala.as_deref()), 16.0)] {
            let Some(texto) = texto else { continue };
            let tx = texto.to_uppercase();
            let lh = self.mono.metricas(fs).2;
            let cw = 4.0 + 36.0 + self.mono.largura(fs, &tx, 1.3);
            let ch = 4.0 + 16.0 + lh;
            ret(pm, x + 30.0, top, cw, ch, 9.0, fundo.a(op));
            borda(pm, x + 30.0, top, cw, ch, 9.0, linha.a(op), 2.0);
            self.mono.desenhar(pm, fs, x + 30.0 + 20.0, self.mono.baseline(fs, top + 10.0, lh), &tx, 1.3, claro.a(op));
            top += ch + 6.0;
        }
    }
    fn cab_magno_larg(&self, cab: &Cabecalho) -> f32 {
        105.0 + self.mono.largura(35.0, &cab.titulo.to_uppercase(), 3.4) + 35.0
    }

    // magno: selo em pilula (foto + @ + "Siga para ver a sua cidade") que e o mesmo objeto do @ do cabecalho:
    // grande em y 445 de 0,2 a 3,6 s, encolhe ate o lado do titulo em 3,6-4,1 s; no fecho volta grande (y 560) em ~0,35 s.
    fn selo_magno(&self, pm: &mut Pixmap, arroba: &str, t: f32) {
        let Some(cab) = &self.r.cabecalho else { return };
        // fecho medido contra o fim da ultima cena (o video nao tem o 0,5 s de respiro do motor)
        let fim = self.r.cenas.iter().map(|c| c.fim as f32).fold(0.0, f32::max);
        let ease = |p: f32| p * p * (3.0 - 2.0 * p);
        // na abertura o selo sobe antes de terminar de encolher (no video, em 3,9 s ja esta na altura do titulo e ainda grande)
        let (grande, tam, gy0) = if t < 4.5 {
            (1.0 - ease(interp(t, &[3.55, 3.95], &[0.0, 1.0])), 1.0 - ease(interp(t, &[3.6, 4.05], &[0.0, 1.0])), 445.0)
        } else {
            let g = ease(interp(t, &[fim - 3.45, fim - 3.1], &[0.0, 1.0]));
            (g, g, 560.0)
        };
        let op = interp(t, &[0.1, 0.25], &[0.0, 1.0]);
        if op <= 0.0 {
            return;
        }
        let frase = "Siga para ver a sua cidade";
        let f1 = &self.arial_neg;
        let w1 = f1.largura(48.0, arroba, 0.0);
        let bw = 155.0 + w1 + 47.0;
        let bh = 140.0;
        // estado pequeno = o grande na metade do tamanho, ao lado da pilula do titulo
        let (px, py) = (MG_CAB.0 + self.cab_magno_larg(cab) + 15.0, MG_CAB.1 + (MG_CAB.2 - bh * 0.5) / 2.0);
        let entra = if t < 1.0 { 0.92 + 0.08 * op } else { 1.0 };
        let s = (0.5 + 0.5 * tam) * entra;
        let (gx, gy) = (540.0 - bw * s / 2.0, gy0 + bh * (1.0 - s) / 2.0);
        let (x, y) = (px + (gx - px) * grande, py + (gy - py) * grande);
        let k = interp(tam, &[0.55, 0.95], &[0.0, 1.0]); // quanto da frase aparece
        ret(pm, x, y, bw * s, bh * s, bh * s / 2.0, Cor::rgba(26, 30, 29, 0.93 * op));
        borda(pm, x, y, bw * s, bh * s, bh * s / 2.0, Cor::hex("#3A3840").a(op), 2.0);
        if let Some(sp) = &self.selo_m {
            let paint = PixmapPaint { opacity: op, quality: FilterQuality::Bilinear, ..Default::default() };
            pm.draw_pixmap(0, 0, sp.as_ref(), &paint, Transform::from_row(s, 0.0, 0.0, s, x + 10.0 * s, y + 10.0 * s), None);
        }
        // baseline do @: 73 com a frase, 87 (centrado) sem ela
        let b1 = 87.0 - 14.0 * k;
        f1.desenhar(pm, 48.0 * s, x + 155.0 * s, y + b1 * s, arroba, 0.0, Cor(1.0, 1.0, 1.0, op));
        if k > 0.0 {
            let w2 = self.arial.largura(26.0, frase, 0.0);
            let fx = 155.0 + w1 / 2.0 - w2 / 2.0;
            self.arial.desenhar(pm, 26.0 * s, x + fx * s, y + 107.0 * s, frase, 0.0, Cor::rgba(150, 220, 180, op * k));
        }
    }

    // magno: o video segura o mapa depois do fim da cena ate o "gastou" da 1a maquete (inicio_ano).
    // Devolve (cena do mapa, opacidade do mapa por cima da maquete; 1 = cobre tudo, cruza em ~0,25 s).
    fn mapa_alem(&self, i: usize, t: f32) -> Option<(&Cena, f32)> {
        let c = &self.r.cenas[i];
        let m = self.r.cenas.get(i.checked_sub(1)?)?;
        if !self.r.magno() || c.peca != "maquete" || m.peca != "mapa" || !c.dados["inicio_ano"].as_bool().unwrap_or(false) {
            return None;
        }
        let ext = c.dados["gastou"]["t"].as_f64().unwrap_or(2.4) as f32;
        let tl = t - c.inicio as f32;
        (tl < ext + 0.2).then(|| (m, interp(tl, &[ext - 0.05, ext + 0.2], &[1.0, 0.0])))
    }

    fn mapa_prolongado(&self, pm: &mut Pixmap, m: &Cena, c: &Cena, t: f32, op: f32) {
        let Some(mut camada) = Pixmap::new(LARG, ALT) else { return };
        let d = &c.dados;
        let tl = t - c.inicio as f32;
        let ta = d["arrecadou"]["t"].as_f64().unwrap_or(1.6) as f32;
        let lf = (t - m.inicio as f32) * FPS as f32;
        let dur = ((m.fim - m.inicio) * FPS).round().max(1.0) as f32;
        let verde = interp(tl, &[ta - 0.5, ta - 0.2], &[0.0, 1.0]);
        crate::mapa::desenhar(&mut camada, &m.dados, lf, dur, crate::mapa::raiz(), self.e.destaque, self.e.texto, true, verde);
        let ano = d["ano"].as_str().unwrap_or("");
        let valor = d["arrecadou"]["rotulo"].as_str().unwrap_or("");
        crate::mapa::fim_magno(&mut camada, ano, valor, tl, ta);
        if let Some(f) = d.get("folha_receita").filter(|f| f["linhas"].as_array().is_some_and(|a| !a.is_empty())) {
            let fop = interp(tl, &[0.0, 0.15], &[0.0, 1.0]);
            folha_magno(&mut camada, f, fop, 98.0, 700.0, 280.0, false);
        }
        pm.draw_pixmap(0, 0, camada.as_ref(), &PixmapPaint { opacity: op, ..Default::default() }, Transform::identity(), None);
    }

    // vaivem de protetor de tela, periodos 11 s e 16 s, opacidade 0,16
    fn marca_dagua(&self, pm: &mut Pixmap, texto: &str, t: f32) {
        let tri = |p: f32| { let p = p % 2.0; if p < 1.0 { p } else { 2.0 - p } };
        let x = 20.0 + (1080.0 - 290.0) * tri(2.0 * t / 11.0);
        let y = 80.0 + (1920.0 - 160.0) * tri(2.0 * t / 16.0);
        let lh = self.neg.metricas(30.0).2;
        self.neg.desenhar(pm, 30.0, x, self.neg.baseline(30.0, y, lh), texto, 0.0, Cor::hex(BRANCO_MESA).a(0.16));
    }

    // selo grande da abertura (0 a 3,5 s) e do fecho (ultimos 3,5 s)
    fn selo_grande(&self, pm: &mut Pixmap, arroba: &str, t: f32) {
        let fim = self.total as f32 / FPS as f32;
        let (abertura, fecho) = (t < 3.5, t > fim - 3.5);
        if !abertura && !fecho {
            return;
        }
        let p = if abertura { interp(t, &[3.0, 3.5], &[0.0, 1.0]) } else { interp(t, &[fim - 3.5, fim - 3.0], &[1.0, 0.0]) };
        if p >= 1.0 {
            return;
        }
        let s = 1.0 - 0.7 * p;
        let dy = -760.0 * p;
        let op = if abertura { 1.0 } else { interp(t, &[fim - 0.9, fim], &[1.0, 0.0]) };
        let frase = "Siga para ver a sua cidade";
        let (lh1, lh2) = (self.arial_neg.metricas(52.0).2, self.arial.metricas(28.0).2);
        let (w1, w2) = (self.arial_neg.largura(52.0, arroba, 0.0), self.arial.largura(28.0, frase, 0.0));
        let bw = w1.max(w2).max(140.0) + 140.0;
        let bh = 80.0 + 140.0 + 18.0 + lh1 + 18.0 + lh2;
        let (cx, cy) = (540.0, 960.0);
        let (bx, by) = (cx - bw / 2.0, cy - bh / 2.0);
        // transform-origin no centro da caixa: p' = c + (p - c) * s + dy
        let m = |x: f32, y: f32| (cx + (x - cx) * s, cy + (y - cy) * s + dy);
        let (x0, y0) = m(bx, by);
        ret(pm, x0, y0, bw * s, bh * s, 28.0 * s, Cor::rgba(14, 20, 17, 0.9 * op));
        if let Some(sp) = &self.selo_g {
            let (sx, sy) = m(cx - 70.0, by + 40.0);
            let paint = PixmapPaint { opacity: op, quality: FilterQuality::Bilinear, ..Default::default() };
            pm.draw_pixmap(0, 0, sp.as_ref(), &paint, Transform::from_row(s, 0.0, 0.0, s, sx, sy), None);
        }
        let topo1 = by + 40.0 + 140.0 + 18.0;
        let (tx, ty) = m(cx - w1 / 2.0, self.arial_neg.baseline(52.0, topo1, lh1));
        self.arial_neg.desenhar(pm, 52.0 * s, tx, ty, arroba, 0.0, Cor(1.0, 1.0, 1.0, op));
        let (tx, ty) = m(cx - w2 / 2.0, self.arial.baseline(28.0, topo1 + lh1 + 18.0, lh2));
        self.arial.desenhar(pm, 28.0 * s, tx, ty, frase, 0.0, Cor::rgba(150, 220, 180, op));
    }

    // legenda karaoke na faixa de baixo (Legenda.tsx)
    fn legenda(&self, pm: &mut Pixmap, t: f64) {
        let Some(fala) = self.r.legenda.iter().find(|f| t >= f.inicio && t < f.fim) else { return };
        let (px, esp, lh, max) = (56.0, -2.9, 56.0 * 1.18, 780.0);
        let f = &self.neg;
        let ativa = self.e.karaoke_ativa.unwrap_or(self.e.texto);
        // cada pedaco: (texto, cor, largura com margem)
        let mut pedacos: Vec<(String, Cor, f32)> = Vec::new();
        let karaoke = self.e.karaoke && !fala.palavras.is_empty();
        if karaoke {
            for p in juntar_numeros(&fala.palavras) {
                let cor = if t >= p.inicio { ativa } else { Cor::hex("#7D7973") };
                let w = f.largura(px, &p.texto, esp) + 20.0;
                pedacos.push((p.texto, cor, w));
            }
        }
        let linhas: Vec<Vec<(String, Cor, f32)>> = if karaoke {
            let mut ls: Vec<Vec<_>> = vec![vec![]];
            let mut larg = 0.0;
            for pd in pedacos {
                if larg + pd.2 > max && !ls.last().unwrap().is_empty() {
                    ls.push(vec![]);
                    larg = 0.0;
                }
                larg += pd.2;
                ls.last_mut().unwrap().push(pd);
            }
            ls
        } else {
            f.quebrar(px, &fala.texto, esp, max).into_iter().map(|l| { let w = f.largura(px, &l, esp); vec![(l, self.e.texto, w)] }).collect()
        };
        let largs: Vec<f32> = linhas.iter().map(|l| l.iter().map(|p| p.2).sum()).collect();
        let bw = largs.iter().cloned().fold(0.0, f32::max) + 60.0;
        let bh = linhas.len() as f32 * lh + 36.0;
        let (bx, by) = (540.0 - bw / 2.0, 1620.0 - bh);
        ret(pm, bx, by, bw, bh, 18.0, Cor::rgba(18, 17, 21, 0.82));
        for (i, l) in linhas.iter().enumerate() {
            let mut x = 540.0 - largs[i] / 2.0;
            let base = f.baseline(px, by + 18.0 + i as f32 * lh, lh);
            for (texto, cor, w) in l {
                f.desenhar(pm, px, x + if karaoke { 10.0 } else { 0.0 }, base, texto, esp, *cor);
                x += w;
            }
        }
    }
}

struct Extras {
    cour: Fonte,
    courbd: Fonte,
    preta: Fonte,
}
fn extras() -> &'static Extras {
    static E: std::sync::OnceLock<Extras> = std::sync::OnceLock::new();
    E.get_or_init(|| Extras {
        cour: Fonte::abrir("cour.ttf").expect("cour.ttf"),
        courbd: Fonte::abrir("courbd.ttf").expect("courbd.ttf"),
        preta: Fonte::abrir("seguibl.ttf").or_else(|_| Fonte::abrir("segoeuib.ttf")).expect("segoe"),
    })
}

impl Motor {
    // EtiquetaMaquete do Estudio.tsx: pilula do topo (setor falado / destaque / titulo) e a folha "PDF"
    fn etiqueta_maquete(&self, pm: &mut Pixmap, d: &Value, t: f32, dur_s: f32) {
        if d["visao_geral"].as_bool().unwrap_or(false) {
            return;
        }
        let seto = crate::maquete::setores(d, 12);
        let tps = crate::maquete::tempos(&seto, dur_s);
        let i_setor = tps.iter().rposition(|x| t >= *x);
        let destaque = d["destaques"].as_array().and_then(|a| a.iter().filter(|x| x["t"].as_f64().unwrap_or(0.0) as f32 <= t).last());
        let pilula = if let Some(i) = i_setor {
            Some((format!("{}: {}", seto[i].nome, seto[i].rotulo), Cor(1.0, 1.0, 1.0, 1.0), Cor::rgba(16, 16, 20, 0.82)))
        } else if let Some(r) = destaque.and_then(|x| x["rotulo"].as_str()).filter(|r| !r.is_empty()) {
            let fundo = match destaque.and_then(|x| x["cor"].as_str()) {
                Some(c) => Cor::hex(&crate::maquete::cor_ano(Some(c), "#d4a537")),
                None => Cor::rgba(16, 16, 20, 0.82),
            };
            Some((r.to_string(), Cor(1.0, 1.0, 1.0, 1.0), fundo))
        } else {
            d["titulo"].as_str().map(|r| (r.to_string(), Cor::hex("#d8d2c6"), Cor::rgba(70, 68, 72, 0.55)))
        };
        if let Some((texto, cor, fundo)) = pilula {
            let f = &extras().courbd;
            let texto = texto.to_uppercase();
            let px = if self.r.magno() { 35.0 } else { 30.0 }; // no video a pilula e ~17% maior
            let linhas = f.quebrar(px, &texto, 2.0, 940.0 - 48.0);
            let lh = f.metricas(px).2;
            let w = linhas.iter().map(|l| f.largura(px, l, 2.0)).fold(0.0, f32::max) + 48.0;
            let h = lh * linhas.len() as f32 + 20.0;
            ret(pm, 540.0 - w / 2.0, 560.0, w, h, 12.0, fundo);
            if self.r.magno() {
                crate::desenho::borda(pm, 540.0 - w / 2.0, 560.0, w, h, 12.0, Cor::hex("#55525a"), 2.0); // no video a pilula tem borda cinza
            }
            for (i, l) in linhas.iter().enumerate() {
                let lw = f.largura(px, l, 2.0);
                f.desenhar(pm, px, 540.0 - lw / 2.0, f.baseline(px, 570.0 + i as f32 * lh, lh), l, 2.0, cor);
            }
        }
        let folha = match i_setor.and_then(|i| seto[i].folha.clone().map(|f| (f, tps[i]))) {
            Some(x) => Some(x),
            None => d.get("folha_receita").filter(|f| f.is_object()).map(|f| (f.clone(), d["arrecadou"]["t"].as_f64().unwrap_or(0.5) as f32)),
        };
        if let Some((f, desde)) = folha {
            if f["linhas"].as_array().map(|a| !a.is_empty()).unwrap_or(false) {
                if self.r.magno() {
                    // magno: folha pequena no alto a esquerda, fica na tela enquanto o setor esta em foco
                    // a folha da receita sai quando entra o "gastou a mais / sobrou" (como no video)
                    let sai = match (i_setor, d["saldo"]["t"].as_f64()) {
                        (None, Some(st)) => interp(t, &[st as f32, st as f32 + 0.3], &[1.0, 0.0]),
                        _ => 1.0,
                    };
                    let op = interp(t - desde, &[0.0, 0.3], &[0.0, 1.0]) * sai;
                    folha_magno(pm, &f, op, 92.0, 680.0, 270.0, i_setor.is_some());
                } else {
                    folha_pdf(pm, &f, desde, t);
                }
            }
        }
    }
}

// folha "PDF" meio transparente no canto: so ~2 s na tela, entra e sai suave
fn folha_pdf(pm: &mut Pixmap, folha: &Value, desde: f32, t: f32) {
    let op = interp(t - desde, &[0.0, 0.3, 2.0, 2.4], &[0.0, 0.9, 0.9, 0.0]);
    folha_pdf_em(pm, folha, op, 90.0, 1010.0);
}

fn folha_pdf_em(pm: &mut Pixmap, folha: &Value, op: f32, x: f32, y: f32) {
    if op <= 0.0 {
        return;
    }
    let ex = extras();
    let iw = 440.0;
    let w = iw + 36.0;
    let tinta = Cor::hex("#1c1c1c").a(op);
    let titulo = folha["titulo"].as_str().unwrap_or("");
    let tl = ex.preta.quebrar(30.0, titulo, 0.0, iw);
    let tlh = 33.0;
    let linhas: Vec<(String, String)> = folha["linhas"].as_array().map(|a| a.iter().take(8).map(|l| (txt_v(&l[0]), valor_br(&l[1]))).collect()).unwrap_or_default();
    let rlh = 13.0 * 1.45;
    let total = match &folha["total"] {
        Value::Null => None,
        v => Some(format!("TOTAL {}", valor_br(v))),
    };
    let flh = ex.courbd.metricas(15.0).2;
    let h = 10.0 + 14.0 + tl.len() as f32 * tlh + 17.0 + linhas.len() as f32 * (rlh + 1.0) + total.as_ref().map(|_| 6.0 + flh).unwrap_or(0.0) + 14.0;
    ret(pm, x, y, w, 30.0, 10.0, Cor::hex("#2FA36B").a(op));
    let papel = Cor::rgba(250, 248, 242, 0.9 * op);
    ret(pm, x, y + 10.0, w, h - 10.0, 10.0, papel);
    ret(pm, x, y + 10.0, w, 12.0, 0.0, papel);
    let mut cy = y + 10.0 + 14.0;
    let cx = x + 18.0;
    for l in &tl {
        ex.preta.desenhar(pm, 30.0, cx, ex.preta.baseline(30.0, cy, tlh), l, 0.0, tinta);
        cy += tlh;
    }
    cy += 8.0;
    ret(pm, cx, cy, iw, 1.0, 0.0, Cor::hex("#cccccc").a(op));
    cy += 9.0;
    for (nome, valor) in &linhas {
        let vw = ex.courbd.largura(13.0, valor, 0.0);
        let nome = reticencias(&ex.cour, 13.0, nome, iw - vw - 10.0);
        ex.cour.desenhar(pm, 13.0, cx, ex.cour.baseline(13.0, cy, rlh), &nome, 0.0, tinta);
        ex.courbd.desenhar(pm, 13.0, cx + iw - vw, ex.courbd.baseline(13.0, cy, rlh), valor, 0.0, tinta);
        cy += rlh;
        ret(pm, cx, cy, iw, 1.0, 0.0, Cor::hex("#dddddd").a(op));
        cy += 1.0;
    }
    if let Some(tt) = total {
        cy += 6.0;
        let tw = ex.courbd.largura(15.0, &tt, 0.0);
        ex.courbd.desenhar(pm, 15.0, cx + iw - tw, ex.courbd.baseline(15.0, cy, flh), &tt, 0.0, tinta);
    }
}

/// magno: folha do balanco legivel, desenhada direto no tamanho final (como no video): cabecalho vermelho,
/// total grande, descricao completa de cada acao (quebra em ate 3 linhas) e valor em negrito a direita
fn folha_magno(pm: &mut Pixmap, folha: &Value, op: f32, x: f32, y: f32, w: f32, onde_foi: bool) {
    if op <= 0.0 {
        return;
    }
    let (reg, neg) = fontes_folha();
    let titulo = folha["titulo"].as_str().unwrap_or("").to_uppercase();
    let cab = if onde_foi { format!("{titulo} · ONDE FOI") } else { titulo.replacen("RECEITA ", "RECEITA ARRECADADA · ", 1) };
    let total = match &folha["total"] {
        Value::Null => String::new(),
        v => valor_br(v),
    };
    let (pad, iw) = (12.0, w - 24.0);
    let (pc, pt, pd, pr) = (11.0, 21.0, 11.0, 12.0);
    let (lc, lt, ld, lr) = (pc * 1.25, pt * 1.15, pd * 1.3, pr * 1.25);
    let cab_l = neg.quebrar(pc, &cab, 0.0, iw);
    let dest_l = folha["destaque"].as_str().map(|d| reg.quebrar(pd, d, 0.0, iw)).unwrap_or_default();
    let linhas: Vec<(Vec<String>, String, bool)> = folha["linhas"]
        .as_array()
        .map(|a| {
            a.iter()
                .take(11)
                .map(|l| {
                    let (nome, valor) = (txt_v(&l[0]), valor_br(&l[1]).trim_start_matches("R$ ").to_string()); // como no video: sem "R$" nas linhas
                    let vw = neg.largura(pr, &valor, 0.0);
                    let mut q = reg.quebrar(pr, &nome, 0.0, iw - vw - 10.0);
                    if q.len() > 3 {
                        q.truncate(3);
                        q[2] = reticencias(reg, pr, &(q[2].clone() + "…"), iw - vw - 10.0);
                    }
                    let outras = nome.starts_with("Outras");
                    (q, valor, outras)
                })
                .collect()
        })
        .unwrap_or_default();
    let h = pad + cab_l.len() as f32 * lc + 2.0 + lt + 6.0 + dest_l.len() as f32 * ld + 6.0
        + linhas.iter().map(|(q, _, _)| q.len() as f32 * lr + 7.0).sum::<f32>()
        + pad;
    ret(pm, x + 2.0, y + 3.0, w, h, 4.0, Cor::rgba(0, 0, 0, 0.25 * op));
    ret(pm, x, y, w, h, 4.0, Cor::rgba(252, 251, 247, op));
    // como no video: despesa (ONDE FOI) em vermelho, receita em verde
    let tom = Cor::hex(if onde_foi { "#c8432e" } else { "#2FA36B" }).a(op);
    ret(pm, x, y, w, 4.0, 0.0, tom);
    let tinta = Cor::hex("#151515").a(op);
    let cinza = Cor::hex("#8d887f").a(op);
    let mut cy = y + pad;
    for l in &cab_l {
        neg.desenhar(pm, pc, x + pad, neg.baseline(pc, cy, lc), l, 0.0, tom);
        cy += lc;
    }
    cy += 2.0;
    neg.desenhar(pm, pt, x + pad, neg.baseline(pt, cy, lt), &total, 0.0, tinta);
    cy += lt + 6.0;
    for l in &dest_l {
        neg.desenhar(pm, pd, x + pad, reg.baseline(pd, cy, ld), l, 0.0, Cor::hex("#3a3a3a").a(op));
        cy += ld;
    }
    cy += 6.0;
    for (q, valor, outras) in &linhas {
        ret(pm, x + pad, cy, iw, 1.0, 0.0, Cor::hex("#e2ded6").a(op));
        cy += 4.0;
        let cor = if *outras { cinza } else { tinta };
        let vw = neg.largura(pr, valor, 0.0);
        neg.desenhar(pm, pr, x + pad + iw - vw, neg.baseline(pr, cy, lr), valor, 0.0, cor);
        for l in q {
            reg.desenhar(pm, pr, x + pad, reg.baseline(pr, cy, lr), l, 0.0, cor);
            cy += lr;
        }
        cy += 3.0;
    }
}

fn fontes_folha() -> &'static (Fonte, Fonte) {
    static F: std::sync::OnceLock<(Fonte, Fonte)> = std::sync::OnceLock::new();
    F.get_or_init(|| (Fonte::abrir("arial.ttf").expect("arial.ttf"), Fonte::abrir("arialbd.ttf").expect("arialbd.ttf")))
}

fn txt_v(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v => v.to_string(),
    }
}
// valorBR: numero vira "R$ 1.234,56"; texto fica como esta
fn valor_br(v: &Value) -> String {
    match v.as_f64() {
        Some(n) if v.is_number() => format!("R$ {}", br(n, 2)),
        _ => txt_v(v),
    }
}
// text-overflow: ellipsis
fn reticencias(f: &Fonte, px: f32, s: &str, max: f32) -> String {
    if f.largura(px, s, 0.0) <= max {
        return s.to_string();
    }
    let mut c: Vec<char> = s.chars().collect();
    while !c.is_empty() && f.largura(px, &(c.iter().collect::<String>() + "…"), 0.0) > max {
        c.pop();
    }
    c.iter().collect::<String>() + "…"
}

// amostras por pixel do 3D (2 = 4 amostras); ESTUDIO_SS=1 deixa mais rapido e serrilhado
fn superamostra() -> usize {
    static S: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *S.get_or_init(|| std::env::var("ESTUDIO_SS").ok().and_then(|v| v.parse().ok()).filter(|v| (1..=4).contains(v)).unwrap_or(2))
}

fn txt(d: &Value, k: &str) -> String {
    match &d[k] {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        v => v.to_string(),
    }
}

// coluna de textos centralizados (com quebra), escalada em volta do proprio centro
struct Item<'a> {
    f: &'a Fonte,
    px: f32,
    lh: Option<f32>,
    cor: Cor,
    mt: f32,
    txt: String,
    brilho: Option<(Cor, f32)>,
}

fn coluna_altura(itens: &[Item], max: f32) -> f32 {
    itens.iter().map(|it| it.mt + it.lh.unwrap_or(it.f.metricas(it.px).2) * it.f.quebrar(it.px, &it.txt, 0.0, max).len() as f32).sum()
}

fn coluna(pm: &mut Pixmap, itens: &[Item], max: f32, topo: f32, escala: f32, opac: f32) {
    let cx = LARG as f32 / 2.0;
    let cy = topo + coluna_altura(itens, max) / 2.0;
    let mut y = topo;
    for it in itens {
        y += it.mt;
        let lh = it.lh.unwrap_or(it.f.metricas(it.px).2);
        for l in it.f.quebrar(it.px, &it.txt, 0.0, max) {
            let w = it.f.largura(it.px, &l, 0.0);
            let base = it.f.baseline(it.px, y, lh);
            if let Some((c, b)) = it.brilho {
                it.f.brilho(pm, it.px * escala, cx - w * escala / 2.0, cy + (base - cy) * escala, &l, 0.0, c.a(opac), b * escala);
            }
            it.f.desenhar(pm, it.px * escala, cx - w * escala / 2.0, cy + (base - cy) * escala, &l, 0.0, it.cor.a(opac));
            y += lh;
        }
    }
}

// linear-gradient(160deg, A, B) do CSS
fn fundo(e: &Estilo) -> Pixmap {
    let mut pm = Pixmap::new(LARG, ALT).expect("pixmap");
    let ang = 160f32.to_radians();
    let (dx, dy) = (ang.sin(), -ang.cos());
    let comp = LARG as f32 * dx.abs() + ALT as f32 * dy.abs();
    let (cx, cy) = (LARG as f32 / 2.0, ALT as f32 / 2.0);
    let sh = LinearGradient::new(
        Point::from_xy(cx - dx * comp / 2.0, cy - dy * comp / 2.0),
        Point::from_xy(cx + dx * comp / 2.0, cy + dy * comp / 2.0),
        vec![GradientStop::new(0.0, e.fundo_a.color()), GradientStop::new(1.0, e.fundo_b.color())],
        SpreadMode::Pad,
        Transform::identity(),
    );
    let mut p = Paint::default();
    if let Some(sh) = sh {
        p.shader = sh;
    }
    pm.fill_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, LARG as f32, ALT as f32).unwrap(), &p, Transform::identity(), None);
    pm
}

// foto redonda dentro do anel do Instagram (SeloFoto)
fn selo(foto: &image::RgbaImage, tam: u32) -> Pixmap {
    let mut pm = Pixmap::new(tam, tam).expect("pixmap");
    let r = tam as f32 / 2.0;
    let ang = 45f32.to_radians();
    let (dx, dy) = (ang.sin(), -ang.cos());
    let comp = tam as f32 * (dx.abs() + dy.abs());
    let paradas = ["#FEDA75", "#FA7E1E", "#D62976", "#962FBF", "#4F5BD5"]
        .iter()
        .enumerate()
        .map(|(i, c)| GradientStop::new(i as f32 / 4.0, Cor::hex(c).color()))
        .collect();
    let mut p = Paint::default();
    p.anti_alias = true;
    if let Some(sh) = LinearGradient::new(
        Point::from_xy(r - dx * comp / 2.0, r - dy * comp / 2.0),
        Point::from_xy(r + dx * comp / 2.0, r + dy * comp / 2.0),
        paradas,
        SpreadMode::Pad,
        Transform::identity(),
    ) {
        p.shader = sh;
    }
    if let Some(c) = PathBuilder::from_circle(r, r, r) {
        pm.fill_path(&c, &p, FillRule::Winding, Transform::identity(), None);
    }
    // object-fit: cover -> recorte quadrado do centro
    let ri = r - tam as f32 * 0.07;
    let d = (ri * 2.0).round().max(1.0) as u32;
    let lado = foto.width().min(foto.height());
    let rec = image::imageops::crop_imm(foto, (foto.width() - lado) / 2, (foto.height() - lado) / 2, lado, lado).to_image();
    let img = image::imageops::resize(&rec, d, d, image::imageops::FilterType::Lanczos3);
    let off = r - d as f32 / 2.0;
    let dados = pm.data_mut();
    for y in 0..d {
        for x in 0..d {
            let (fx, fy) = (off + x as f32 + 0.5, off + y as f32 + 0.5);
            let cob = (ri - ((fx - r).powi(2) + (fy - r).powi(2)).sqrt() + 0.5).clamp(0.0, 1.0);
            let (px, py) = (fx as u32, fy as u32);
            if cob <= 0.0 || px >= tam || py >= tam {
                continue;
            }
            let c = img.get_pixel(x, y).0;
            let i = ((py * tam + px) * 4) as usize;
            for k in 0..3 {
                dados[i + k] = (c[k] as f32 * cob + dados[i + k] as f32 * (1.0 - cob)) as u8;
            }
            dados[i + 3] = (255.0 * cob + dados[i + 3] as f32 * (1.0 - cob)) as u8;
        }
    }
    if let Some(c) = PathBuilder::from_circle(r, r, ri - 1.0) {
        let st = Stroke { width: 2.0, ..Default::default() };
        pm.stroke_path(&c, &Cor::hex("#16151A").paint(), &st, Transform::identity(), None);
    }
    pm
}


// --- comparativo do visual magno (recriado olhando o video) ---
fn e_comparativo(c: &Cena) -> bool {
    c.peca == "barras" && c.dados["mesa"].as_bool().unwrap_or(false)
}

// "EVENTOS DA SALTUR: PREVIA x GASTOU" -> "EVENTOS DA SALTUR"
fn assunto(titulo: &str) -> String {
    let fim = [titulo.find(':'), titulo.find(" x "), titulo.find(" X ")].into_iter().flatten().min().unwrap_or(titulo.len());
    titulo[..fim].trim().to_uppercase()
}

/// No video a faixa e uma so: as colunas das comparacoes anteriores ficam de pe e as novas entram ao lado.
/// Junta as colunas das cenas anteriores (ja crescidas, t negativo), sem repetir a mesma coluna, e o rotulo com todos os assuntos.
fn comparativo_acumulado(cenas: &[Cena], atual: &Cena) -> Value {
    let comps: Vec<&Cena> = cenas.iter().filter(|c| e_comparativo(c)).collect();
    let mut valores = Vec::new();
    let mut rotulos = Vec::new();
    let mut cores = Vec::new();
    let mut barras = Vec::new();
    let mut vistos: Vec<(String, String)> = Vec::new();
    let mut max: f64 = 1.0;
    for c in &comps {
        for v in c.dados["valores"].as_array().into_iter().flatten() {
            max = max.max(v.as_f64().unwrap_or(0.0));
        }
    }
    for c in comps.iter().take_while(|c| !std::ptr::eq(**c, atual)).chain(std::iter::once(&atual)) {
        let d = &c.dados;
        let eh_atual = std::ptr::eq(*c, atual);
        for (i, v) in d["valores"].as_array().into_iter().flatten().enumerate() {
            let chave = (format!("{:.2}", v.as_f64().unwrap_or(0.0)), d["cores"][i].as_str().unwrap_or("").to_string());
            if let Some(j) = vistos.iter().position(|x| *x == chave) {
                if eh_atual {
                    // coluna que ja existe (ex.: os eventos de novo): so troca a plaquinha pelo rotulo novo
                    rotulos[j] = d["rotulos"][i].clone();
                }
                continue;
            }
            vistos.push(chave);
            valores.push(v.clone());
            rotulos.push(d["rotulos"][i].clone());
            cores.push(d["cores"][i].clone());
            let t = if eh_atual { d["barras"][i]["t"].as_f64().unwrap_or(0.3 + i as f64 * 0.8) } else { -100.0 };
            barras.push(serde_json::json!({ "t": t }));
        }
    }
    let faixa = comps.iter().map(|c| assunto(c.dados["titulo"].as_str().unwrap_or(""))).filter(|s| !s.is_empty()).fold(Vec::<String>::new(), |mut a, s| {
        if !a.contains(&s) {
            a.push(s);
        }
        a
    });
    // total de colunas de TODOS os comparativos: a posicao de cada coluna na faixa nao muda de uma cena para a outra
    let mut todas: Vec<(String, String)> = Vec::new();
    for c in &comps {
        for (i, v) in c.dados["valores"].as_array().into_iter().flatten().enumerate() {
            let chave = (format!("{:.2}", v.as_f64().unwrap_or(0.0)), c.dados["cores"][i].as_str().unwrap_or("").to_string());
            if !todas.contains(&chave) {
                todas.push(chave);
            }
        }
    }
    let mut d = atual.dados.clone();
    d["faixa_total"] = Value::from(todas.len());
    d["valores"] = Value::from(valores);
    d["rotulos"] = Value::from(rotulos);
    d["cores"] = Value::from(cores);
    d["barras"] = Value::from(barras);
    d["max_faixa"] = Value::from(max);
    // unidade dos valores (o roteiro traz a faixa em milhoes): lida do 1o rotulo, para a conta das moedas ser em reais
    let un = comps.first().map(|c| crate::maquete_magno::unidade_do_rotulo(c.dados["valores"][0].as_f64().unwrap_or(0.0), c.dados["rotulos"][0].as_str().unwrap_or(""))).unwrap_or(1.0);
    d["faixa_unidade"] = Value::from(un);
    d["faixa_magno"] = Value::from(faixa.join(" · "));
    d
}

/// pilula do alto: o titulo (escura) ate a primeira coluna nascer, depois o rotulo da ultima coluna em verde
fn pilula_comparativo(pm: &mut Pixmap, c: &Cena, t: f32) {
    let d = &c.dados;
    let ultima = (0..d["valores"].as_array().map(|a| a.len()).unwrap_or(0)).filter(|&i| d["barras"][i]["t"].as_f64().unwrap_or(0.0) as f32 <= t).last();
    let (texto, fundo) = match ultima {
        Some(i) => {
            let r = d["rotulos"][i].as_str().unwrap_or("").to_uppercase();
            // "2024 GASTOU R$ 219 MI" -> "2024: GASTOU R$ 219 MI", como no video
            let r = if r.len() > 5 && r[..4].chars().all(|x| x.is_ascii_digit()) && &r[4..5] == " " { format!("{}:{}", &r[..4], &r[4..]) } else { r };
            (r, Cor::hex("#2fa36b"))
        }
        None => (d["titulo"].as_str().unwrap_or("").to_uppercase(), Cor::rgba(16, 16, 20, 0.82)),
    };
    if texto.is_empty() {
        return;
    }
    let f = &extras().courbd;
    let px = 30.0;
    let lh = f.metricas(px).2;
    let w = f.largura(px, &texto, 2.0).min(940.0) + 48.0;
    let h = lh + 20.0;
    ret(pm, 540.0 - w / 2.0, 560.0, w, h, 12.0, fundo);
    crate::desenho::borda(pm, 540.0 - w / 2.0, 560.0, w, h, 12.0, Cor::hex("#55525a"), 2.0);
    let lw = f.largura(px, &texto, 2.0);
    f.desenhar(pm, px, 540.0 - lw / 2.0, f.baseline(px, 570.0, lh), &texto, 2.0, Cor(1.0, 1.0, 1.0, 1.0));
}
