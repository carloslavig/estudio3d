// MAQUETE e COMPARATIVO (mesa redonda, cofre, casinhas, pilhas de moedas, plaquinhas) do src/pecas/Maquete.tsx, sobre o motor 3D de tres.rs.
use crate::desenho::{interp, mola, ret, Cor, Fonte};
use crate::tres::{v3, Cena3, Mat, Tex, M4, V3};
use serde_json::Value;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex, OnceLock};

const TOPO: f32 = -1.3;
const RAIO_ANEL: f32 = 5.0;
const FPS: f32 = 30.0;
/// camera do visual magno: [desce no foco, aproxima no foco, inclinacao extra no foco, escala extra no foco, desce geral, aproxima geral, escala geral]
/// MOTOR_CAM="a,b,c,d,e,f,g" troca os valores (so para afinar comparando com o video)
fn cam_mg() -> [f32; 7] {
    static C: OnceLock<[f32; 7]> = OnceLock::new();
    *C.get_or_init(|| {
        let mut c = [2.8, 5.0, -0.1, 0.3, -0.4, 0.0, 0.45];
        if let Ok(v) = std::env::var("MOTOR_CAM") {
            for (i, x) in v.split(',').take(7).enumerate() {
                if let Ok(x) = x.trim().parse() {
                    c[i] = x;
                }
            }
        }
        c
    })
}

pub fn cor_ano(c: Option<&str>, padrao: &'static str) -> String {
    let tabela = [
        ("dourado", "#d4a537"),
        ("verde", "#3aa676"),
        ("rosa", "#d8707f"),
        ("rosa claro", "#efb3bd"),
        ("dourado claro", "#ead08f"),
        ("verde claro", "#9fd8b9"),
        ("azul", "#4aa3ff"),
        ("prata", "#cfd6e0"),
        ("vermelho", "#ef4444"),
    ];
    match c {
        Some(c) => tabela.iter().find(|(k, _)| *k == c).map(|(_, v)| v.to_string()).unwrap_or_else(|| {
            if c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit()) { c.to_string() } else { padrao.to_string() }
        }),
        None => padrao.to_string(),
    }
}

#[derive(Clone)]
pub struct Setor {
    pub nome: String,
    pub valor: Option<f64>,
    pub rotulo: String,
    pub tipo: Option<String>,
    pub zerado: bool,
    pub t: Option<f32>,
    pub anteriores: Vec<(String, String, Option<String>)>, // ano, rotulo, cor
    pub folha: Option<Value>,
}

pub fn setores(d: &Value, max: usize) -> Vec<Setor> {
    d["setores"]
        .as_array()
        .map(|a| {
            a.iter()
                .take(max)
                .map(|s| Setor {
                    nome: s["nome"].as_str().unwrap_or("").to_string(),
                    valor: s["valor"].as_f64(),
                    rotulo: s["rotulo"].as_str().unwrap_or("").to_string(),
                    tipo: s["tipo"].as_str().map(String::from),
                    zerado: s["zerado"].as_bool().unwrap_or(false),
                    t: s["t"].as_f64().map(|x| x as f32),
                    anteriores: s["anteriores"]
                        .as_array()
                        .map(|v| v.iter().map(|a| (a["ano"].as_str().unwrap_or("").to_string(), a["rotulo"].as_str().unwrap_or("").to_string(), a["cor"].as_str().map(String::from))).collect())
                        .unwrap_or_default(),
                    folha: s.get("folha").cloned(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// instante (s) em que cada setor entra: o "t" dado, ou espalhado igualmente na cena
pub fn tempos(s: &[Setor], duracao: f32) -> Vec<f32> {
    let n = s.len().max(1) as f32;
    let passo = ((duracao - 3.5) / n).max(1.2);
    s.iter().enumerate().map(|(i, x)| x.t.unwrap_or(1.2 + i as f32 * passo)).collect()
}

fn suave(x: f32, a: f32, b: f32) -> f32 {
    let t = interp(x, &[a, b], &[0.0, 1.0]);
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

// ---------- texturas das plaquinhas (canvas do Maquete.tsx), com cache ----------

pub(crate) struct Fontes {
    pub(crate) arial_black: Fonte,
    pub(crate) courier: Fonte,
}
pub(crate) fn fontes() -> &'static Fontes {
    static F: OnceLock<Fontes> = OnceLock::new();
    // shortcut: "900 Arial" no Chrome do Windows cai na Arial Black (familia Arial do DirectWrite); conferido nos quadros
    F.get_or_init(|| Fontes { arial_black: Fonte::abrir("ariblk.ttf").expect("ariblk.ttf"), courier: Fonte::abrir("courbd.ttf").expect("courbd.ttf") })
}
fn cache() -> &'static Mutex<HashMap<String, Arc<Tex>>> {
    static C: OnceLock<Mutex<HashMap<String, Arc<Tex>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}
pub(crate) fn com_cache(chave: String, fazer: impl FnOnce() -> tiny_skia::Pixmap) -> Arc<Tex> {
    if let Some(t) = cache().lock().unwrap().get(&chave) {
        return t.clone();
    }
    let t = Arc::new(Tex::de_pixmap(&fazer()));
    cache().lock().unwrap().insert(chave, t.clone());
    t
}

fn placa(l1: &str, l2: &str, cor: &str) -> Arc<Tex> {
    com_cache(format!("placa|{l1}|{l2}|{cor}"), || {
        let (mut a, mut b) = (l1.to_string(), l2.to_string());
        if b.is_empty() {
            if let Some(i) = a.find(" R$") {
                // achado: valor grande em cima, nome embaixo
                b = a[..i].to_string();
                a = a[i + 1..].to_string();
            }
        }
        let mut pm = tiny_skia::Pixmap::new(512, 190).unwrap();
        ret(&mut pm, 6.0, 6.0, 500.0, 178.0, 26.0, Cor::hex(cor));
        let f = &fontes().arial_black;
        let mut tam = 74.0;
        let mut usado;
        loop {
            usado = tam;
            tam -= 4.0;
            if !(f.largura(usado, &a, 0.0) > 470.0 && tam > 28.0) {
                break;
            }
        }
        let w = f.largura(usado, &a, 0.0);
        f.desenhar(&mut pm, usado, 256.0 - w / 2.0, 96.0, &a, 0.0, Cor(1.0, 1.0, 1.0, 1.0));
        let mut tam = 40.0;
        loop {
            usado = tam;
            tam -= 3.0;
            if !(f.largura(usado, &b, 0.0) > 470.0 && tam > 20.0) {
                break;
            }
        }
        let w = f.largura(usado, &b, 0.0);
        f.desenhar(&mut pm, usado, 256.0 - w / 2.0, 158.0, &b, 0.0, Cor(1.0, 1.0, 1.0, 0.85));
        pm
    })
}

fn faixa_titulo(titulo: &str) -> Arc<Tex> {
    com_cache(format!("faixa|{titulo}"), || {
        let mut pm = tiny_skia::Pixmap::new(1024, 96).unwrap();
        ret(&mut pm, 0.0, 0.0, 1024.0, 96.0, 0.0, Cor::hex("#1f6b40"));
        let t = titulo.to_uppercase();
        let px = if titulo.chars().count() > 30 { 40.0 } else { 52.0 };
        let f = &fontes().courier;
        let w = f.largura(px, &t, 0.0);
        f.desenhar(&mut pm, px, 512.0 - w / 2.0, 66.0, &t, 0.0, Cor(1.0, 1.0, 1.0, 1.0));
        pm
    })
}

/// magno: o titulo da faixa em pedacos de 1024 px emendados (a frase inteira nao cabe numa textura so e saia cortada).
/// Pedaco k da frase; `titulo` vazio = so o fundo. Letra no tamanho natural, comecando na ponta esquerda
fn faixa_titulo_parte(titulo: &str, k: usize) -> Arc<Tex> {
    com_cache(format!("faixa-parte|{titulo}|{k}"), || {
        let mut pm = tiny_skia::Pixmap::new(1024, 96).unwrap();
        ret(&mut pm, 0.0, 0.0, 1024.0, 96.0, 0.0, Cor::hex("#1f6b40"));
        fontes().courier.desenhar(&mut pm, 52.0, 28.0 - k as f32 * 1024.0, 66.0, &titulo.to_uppercase(), 0.0, Cor(1.0, 1.0, 1.0, 1.0));
        pm
    })
}

/// quantos pedacos de 1024 px a frase ocupa
fn faixa_titulo_partes(titulo: &str) -> usize {
    ((28.0 + fontes().courier.largura(52.0, &titulo.to_uppercase(), 0.0)) / 1024.0).ceil() as usize
}

#[cfg(test)]
mod teste_titulo {
    #[test]
    fn frase_longa_da_faixa_cabe_inteira_nos_pedacos() {
        let t = "EVENTOS DA SALTUR · PUBLICIDADE · ALIMENTAÇÃO ESCOLAR";
        let n = super::faixa_titulo_partes(t);
        let w = super::fontes().courier.largura(52.0, t, 0.0);
        assert!(n >= 2 && 28.0 + w <= n as f32 * 1024.0, "{n} pedacos para {w} px");
    }
}

// ---------- pecas ----------

/// magno: pilha de moedas soltas, cada uma um tiquinho deslocada (empilhada a mao, da para contar)
pub(crate) static MOEDAS_SOLTAS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn pilha_solta(c: &mut Cena3, m: &M4, pos: V3, cor: &str, alt: f32) {
    pilha_moedas(c, m, pos, cor, 0, (alt / 0.042).round().max(1.0) as usize);
}

/// magno: pilha de n moedas de um tamanho (0 grande, 1 media, 2 pequena), cada moeda um tiquinho deslocada
pub(crate) fn pilha_moedas(c: &mut Cena3, m: &M4, pos: V3, cor: &str, tipo: usize, n: usize) {
    let (raio, esp, _) = crate::maquete_magno::MOEDA[tipo];
    let g = m.e(pos, V3::default(), 1.0);
    let lado = c.mat(Mat::std(cor).m(0.5).r(0.38));
    let topo = c.mat(Mat::std(cor).e(cor, 0.25).m(0.5).r(0.3));
    // semente pela posicao: a mesma pilha fica igual em todos os quadros
    let mut s = ((pos.x * 73.1 + pos.z * 191.7).abs() * 1000.0) as u32 | 1;
    let mut acaso = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s % 1000) as f32 / 1000.0 - 0.5
    };
    for i in 0..n {
        let (dx, dz) = if i == 0 { (0.0, 0.0) } else { (acaso() * 0.025, acaso() * 0.025) };
        let y = i as f32 * esp;
        c.cilindro(&g.e(v3(dx, y + (esp - 0.004) / 2.0, dz), V3::default(), 1.0), raio, raio, esp - 0.004, 24, lado); // 24 lados: sao milhares de moedas
        if i + 1 == n {
            c.cilindro(&g.e(v3(dx, y + esp - 0.0035, dz), V3::default(), 1.0), raio - 0.003, raio - 0.003, 0.003, 24, topo);
        }
    }
}

pub(crate) fn pilha(c: &mut Cena3, m: &M4, pos: V3, cor: &str, alt: f32) {
    if MOEDAS_SOLTAS.load(std::sync::atomic::Ordering::Relaxed) {
        return pilha_solta(c, m, pos, cor, alt);
    }
    let g = m.e(pos, V3::default(), 1.0);
    let mut lado = Mat::std("#ffffff").m(0.45).r(0.35);
    lado.cor = crate::tres::hex(cor);
    lado.listras = true;
    let lado = c.mat(lado);
    c.cilindro(&g.e(v3(0.0, alt / 2.0, 0.0), V3::default(), 1.0), 0.13, 0.13, alt, 22, lado);
    let topo = c.mat(Mat::std(cor).e(cor, 0.25).m(0.5).r(0.3));
    c.cilindro(&g.e(v3(0.0, alt + 0.005, 0.0), V3::default(), 1.0), 0.128, 0.128, 0.01, 22, topo);
}

fn casinha(c: &mut Cena3, m: &M4, tipo: &str, zerado: bool) {
    let caixa = |c: &mut Cena3, pos: V3, w: f32, h: f32, d: f32, mat: Mat| {
        let k = c.mat(mat);
        c.caixa(&m.e(pos, V3::default(), 1.0), w, h, d, k);
    };
    match tipo {
        "campo" => {
            caixa(c, v3(0.0, 0.06, 0.0), 1.25, 0.05, 0.8, Mat::std(if zerado { "#8a8a8a" } else { "#2f9e57" }));
            caixa(c, v3(0.0, 0.1, 0.0), 0.02, 0.02, 0.8, Mat::std("#ffffff"));
            for x in [-0.62, 0.62] {
                caixa(c, v3(x, 0.3, 0.0), 0.04, 0.4, 0.3, Mat::std("#ffffff"));
            }
        }
        "rua" => {
            caixa(c, v3(0.0, 0.06, 0.0), 1.3, 0.04, 0.5, Mat::std(if zerado { "#777777" } else { "#3a3f47" }));
            caixa(c, v3(-0.2, 0.28, 0.0), 0.55, 0.3, 0.32, Mat::std(if zerado { "#999999" } else { "#f59e0b" }));
            caixa(c, v3(0.2, 0.22, 0.0), 0.25, 0.2, 0.3, Mat::std(if zerado { "#999999" } else { "#2563eb" }));
        }
        _ => {
            let parede = if zerado { "#8a8a8a" } else { match tipo { "hospital" => "#f1f5f9", "escola" => "#e8c27a", "predio" => "#d9d3c4", _ => "#c9bba0" } };
            let telhado = if zerado { "#6a6a6a" } else if tipo == "hospital" { "#ef4444" } else { "#a8452f" };
            caixa(c, v3(0.0, 0.38, 0.0), 0.9, 0.62, 0.7, Mat::std(parede).r(0.6));
            let k = c.mat(Mat::std(telhado).r(0.55));
            c.cilindro(&m.e(v3(0.0, 0.9, 0.0), v3(0.0, PI / 4.0, 0.0), 1.0), 0.0, 0.72, 0.42, 4, k);
            if tipo == "predio" {
                let k = c.mat(Mat::std("#f5f5f4"));
                for x in [-0.3, -0.1, 0.1, 0.3] {
                    c.cilindro(&m.e(v3(x, 0.36, 0.38), V3::default(), 1.0), 0.045, 0.045, 0.58, 12, k);
                }
            }
            if tipo == "hospital" {
                caixa(c, v3(0.0, 0.5, 0.36), 0.36, 0.1, 0.02, Mat::std("#ef4444"));
                caixa(c, v3(0.0, 0.5, 0.36), 0.1, 0.36, 0.02, Mat::std("#ef4444"));
            }
            if tipo == "escola" {
                caixa(c, v3(0.34, 1.0, 0.2), 0.025, 0.5, 0.025, Mat::std("#cbd5e1"));
                caixa(c, v3(0.43, 1.18, 0.2), 0.18, 0.1, 0.01, Mat::std("#22c55e"));
            }
            caixa(c, v3(0.0, 0.2, 0.355), 0.16, 0.3, 0.01, Mat::std("#5b4636"));
        }
    }
}

pub(crate) fn plano_tex(c: &mut Cena3, m: &M4, w: f32, h: f32, tex: Arc<Tex>) {
    let k = c.mat(Mat::basico(tex));
    c.plano(m, w, h, k);
}

/// monta a cena 3D de um quadro. `lf` = quadro local, `dur` = quadros da cena. Com `gaveta`, e o Comparativo (barras com mesa).
pub fn cena(d: &Value, lf: f32, dur: f32, gaveta: bool, mg: bool) -> Cena3 {
    let mut c = Cena3::default();
    MOEDAS_SOLTAS.store(mg, std::sync::atomic::Ordering::Relaxed);
    // magno: mesa e anel maiores, para cada setor (predio + pilhas) ter espaco proprio sem encostar no vizinho
    let ra = if mg { crate::maquete_magno::ANEL } else { RAIO_ANEL };
    let r_mesa = if mg { crate::maquete_magno::MESA_R } else { 6.8 };
    let frame = lf;
    let t = lf / FPS;
    let seto = if gaveta { vec![] } else { setores(d, 16) };
    let todos: Vec<(String, Option<String>)> = match d["todos"].as_array() {
        Some(a) if !a.is_empty() => a.iter().take(16).map(|x| (x["nome"].as_str().unwrap_or("").to_string(), x["tipo"].as_str().map(String::from))).collect(),
        _ => seto.iter().map(|s| (s.nome.clone(), s.tipo.clone())).collect(),
    };
    let ja: Vec<&str> = d["ja"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
    let ja_ano: HashMap<String, f64> = d["ja_ano"].as_object().map(|o| o.iter().filter_map(|(k, v)| v.as_f64().map(|x| (k.clone(), x))).collect()).unwrap_or_default();
    let cor_do_ano = cor_ano(d["cor"].as_str(), "#d4a537");
    let ano = match &d["ano"] {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    let pilha_mi = d["pilha_mi"].as_f64().filter(|x| *x > 0.0).unwrap_or(2.0);
    let tps = tempos(&seto, dur / FPS);
    let geral = d["visao_geral"].as_bool().unwrap_or(false);
    let i_ativo = tps.iter().rposition(|tt| t >= *tt).map(|i| i as isize).unwrap_or(-1);
    let n = todos.len().max(1) as f32;
    let ang_de = |nome: &str| todos.iter().position(|x| x.0 == nome).unwrap_or(0) as f32 / n * PI * 2.0 - PI / 2.0;
    let saldo = d.get("saldo").filter(|s| !gaveta && s.is_object());
    let volta = d["volta"].as_f64().unwrap_or(0.0) as f32;
    let ordem_ja: Vec<&str> = d["ja_ordem"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).filter(|x| ja_ano.contains_key(*x)).collect()).unwrap_or_default();
    // magno: o setor em foco fica um pouco a direita do centro (a folha do balanco ocupa a esquerda, como no video)
    let giro: f32 = if mg { std::env::var("MOTOR_GIRO").ok().and_then(|v| v.parse().ok()).unwrap_or(-0.18) } else { 0.0 };
    let alvo = |nome: &str| ang_de(nome) + PI / 2.0 + volta * PI * 2.0 + giro;
    let mut yaw = if let Some(u) = ordem_ja.last() { alvo(u) } else if let Some(s) = seto.first() { alvo(&s.nome) - 0.5 } else { volta * PI * 2.0 };
    // magno, visao geral: a camera nao gira (so se afasta); antes ela rodava ate o ultimo setor da lista
    for (i, x) in seto.iter().enumerate().filter(|_| !(mg && geral)) {
        let k = suave(t, tps[i] - 0.2, tps[i] + 1.0);
        yaw += (yaw.max(alvo(&x.nome)) - yaw) * k;
    }
    // faixa e visao geral continuam do ponto em que a cena anterior parou (a deriva dela ja chegou ao fim)
    yaw += if mg && (gaveta || geral) { 0.024 } else { 0.006 * t.min(4.0) };
    let inicio_ano = d["inicio_ano"].as_bool().unwrap_or(false);
    let pitch = if geral { 1.05 } else if inicio_ano { 1.35 + (0.62 - 1.35) * suave(t, 0.0, 2.5) } else { 0.62 };
    let k_zoom = if geral || seto.is_empty() { 0.0 } else if !inicio_ano { 1.0 } else { suave(t, tps[0] - 0.3, tps[0] + 1.2) };
    let gastou_ano = d["gastou_ano"].as_f64().unwrap_or(0.0);
    let falado: f64 = ja_ano.values().sum::<f64>() + seto.iter().enumerate().map(|(i, x)| if t >= tps[i] + 0.4 { x.valor.unwrap_or(0.0) } else { 0.0 }).sum::<f64>();
    let teto_cofre = if mg { 80.0 } else { 30.0 }; // no video do Magno o cofre comeca cercado de muitas pilhas
    let no_cofre = if gastou_ano > 0.0 && !geral { ((gastou_ano / (pilha_mi * 1e6)).min(teto_cofre) * (1.0 - falado / gastou_ano).max(0.0)).round() as usize } else { 0 };

    // magno: quem leva cada pilha do anel do cofre. Ordem fixa no ano (setores ja falados, depois os da cena);
    // cada setor fica com as pilhas mais proximas da direcao dele (a fatia do anel virada para o setor)
    let tg = d["gastou"]["t"].as_f64().unwrap_or(2.4) as f32;
    // cofre girado para a porta ficar de frente para a camera da abertura do ano (fixo no ano todo)
    let primeiro = ordem_ja.first().map(|x| x.to_string()).or_else(|| seto.first().map(|x| x.nome.clone()));
    let rc = primeiro.map(|p| -(alvo(&p) - 0.5)).unwrap_or(0.0) + std::env::var("MOTOR_PORTA").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0f32);
    let porta = M4::ry(rc).p(v3(0.0, 0.0, 0.68));
    use crate::maquete_magno::compor;
    // magno: o anel do cofre E o dinheiro do ano, ja separado: cada setor tem as pilhas dele na fatia do anel virada
    // para ele (moedas grandes, medias e pequenas, pilhas de alturas diferentes) e elas voam inteiras ate o setor.
    // Os valores de todos os setores do ano vem em `valores_ano` (quadro.rs junta as cenas do mesmo ano).
    let valores_ano: HashMap<String, f64> = d["valores_ano"].as_object().map(|o| o.iter().filter_map(|(k, v)| v.as_f64().map(|x| (k.clone(), x))).collect()).unwrap_or_default();
    // (setor, pilhas, estado): estado >= 0 = indice em `seto` (voa nesta cena), -1 = ja esta no setor, -2 = ainda no anel
    let ordem: Vec<(String, Vec<(usize, usize)>, i32)> = todos
        .iter()
        .enumerate()
        .filter(|_| mg)
        .filter_map(|(k, (n, _))| {
            let i = seto.iter().position(|s| &s.nome == n);
            let v = i.and_then(|i| seto[i].valor).or_else(|| ja_ano.get(n).cloned()).or_else(|| valores_ano.get(n).cloned()).filter(|v| *v > 0.0)?;
            let estado = match i {
                Some(i) => i as i32,
                None if ja_ano.contains_key(n) => -1,
                None => -2,
            };
            Some((n.clone(), compor(v, pilha_mi, k), estado))
        })
        .collect();
    // o que o ano gastou alem dos setores conhecidos fica no anel, sem dono
    let conhecido: f64 = todos.iter().filter_map(|(n, _)| seto.iter().find(|s| &s.nome == n).and_then(|s| s.valor).or_else(|| ja_ano.get(n).cloned()).or_else(|| valores_ano.get(n).cloned())).sum();
    let tem_anel = mg && gastou_ano > 0.0 && !geral;
    let sobra = if tem_anel { compor(gastou_ano - conhecido, pilha_mi, 3) } else { Vec::new() };
    let n0 = if tem_anel { ordem.iter().map(|o| o.1.len()).sum::<usize>() + sobra.len() } else { 0 };
    let pos0: Vec<(f32, f32)> = (0..n0).map(|k| crate::maquete_magno::pos_cofre(k, n0)).collect();
    // cada pilha pega o lugar livre mais perto da direcao do setor dela (ordem fixa: o anel e igual o ano todo)
    let mut livre = vec![true; n0];
    let mut pega = |a: Option<f32>| -> usize {
        let dist = |k: usize| match a {
            Some(a) => {
                let d = (pos0[k].1.atan2(pos0[k].0) - a).rem_euclid(2.0 * PI);
                d.min(2.0 * PI - d) + k as f32 * 1e-4
            }
            None => k as f32,
        };
        let k = (0..n0).filter(|k| livre[*k]).min_by(|x, y| dist(*x).total_cmp(&dist(*y))).unwrap_or(0);
        if k < n0 {
            livre[k] = false;
        }
        k
    };
    let lugar: Vec<Vec<usize>> = ordem.iter().map(|(n, ps, _)| if tem_anel { ps.iter().map(|_| pega(Some(ang_de(n)))).collect() } else { Vec::new() }).collect();
    let lugar_sobra: Vec<usize> = sobra.iter().map(|_| pega(None)).collect();
    let de_onde: HashMap<(String, usize), (f32, f32)> = HashMap::new(); // so o visual antigo consulta (sempre vazio)
    // quanto a pilha kk do setor i ja voou (0 = ainda no lugar, 1 = chegou)
    // magno: a pilha so sai quando a camera ja esta virada para o setor (~0,7 s), para ser vista saindo do anel
    let espera = if mg { 0.7 * FPS } else { 6.0 };
    let p_voo = |i: usize, kk: usize| ((frame - tps[i] * FPS - espera - kk as f32 * 2.0) / (0.9 * FPS)).clamp(0.0, 1.0);

    let cam = cam_mg();
    let raiz = if mg {
        // magno: camera mais baixa e mais perto do setor (medido nos quadros do video)
        {
            // visao geral (comeco do ano) bem de cima e de longe, como no video; no foco, a camera do setor
            let k = k_zoom;
            let p_geral = if geral { 1.05 } else if inicio_ano { 1.35 + (0.85 - 1.35) * suave(t, 0.0, 2.5) } else { 0.85 };
            let mix = |a: f32, b: f32| a + (b - a) * k;
            M4::trs(v3(0.0, -0.2 - mix(cam[4], cam[0]), mix(cam[5], cam[1])), v3(mix(p_geral, 0.62 + cam[2]), yaw, 0.0), mix(if geral { 0.42 } else if inicio_ano { cam[6] * (1.0 + 0.3 * suave(t, 0.0, 2.5)) } else { cam[6] }, 0.62 + cam[3]))
        }
    } else {
        M4::trs(v3(0.0, -0.2 - 1.8 * k_zoom, 2.6 * k_zoom), v3(pitch + 0.18 * k_zoom, yaw, 0.0), (if geral { 0.42 } else { 0.62 }) + 0.23 * k_zoom)
    };
    // magno: no comparativo a camera fica baixa e perto da faixa, acompanhando a coluna da vez
    let faixa_ant = d.get("faixa_ant").filter(|f| mg && geral && f.is_object());
    // giro da faixa: a ponta esquerda aponta para o predio que esta mais a esquerda da camera (ela comeca colada nele)
    let psi = if mg {
        let gm = M4::ry(yaw); // do sistema da mesa para o da faixa (o inverso do giro -yaw do grupo)
        todos
            .iter()
            .map(|(n, _)| {
                let a = ang_de(n);
                let q = gm.p(v3(a.cos(), 0.0, a.sin()));
                q.z.atan2(-q.x)
            })
            .min_by(|a, b| a.abs().total_cmp(&b.abs()))
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let fg = -psi; // a camera gira o mesmo tanto: a faixa fica reta na tela
    let raiz = if gaveta && mg {
        // 1a cena da faixa: a camera vem deslizando do ultimo setor (sem corte seco); nas seguintes ja esta la
        let (ft, fp) = faixa_cam(d, frame);
        let k = if d["faixa_primeira"].as_bool().unwrap_or(false) { suave(t, 0.0, 1.5) } else { 1.0 };
        let m = |a: f32, b: f32| a + (b - a) * k;
        M4::trs(v3(m(0.0, ft.x), m(-0.2 - cam[0], ft.y), m(cam[1], ft.z)), v3(m(0.62 + cam[2], fp), yaw + k * fg, 0.0), m(0.62 + cam[3], 1.0))
    } else if let Some(fa) = faixa_ant {
        // visao geral depois da faixa: a camera so se afasta dela (a faixa continua na mesa)
        let (ft, fp) = faixa_cam(fa, d["faixa_ant_frames"].as_f64().unwrap_or(1e5) as f32);
        let k = suave(t, 0.0, 1.8);
        let m = |a: f32, b: f32| a + (b - a) * k;
        M4::trs(v3(m(ft.x, 0.0), m(ft.y, -0.2 - cam[4]), m(ft.z, cam[5])), v3(m(fp, 1.05), yaw + fg, 0.0), m(1.0, 0.42))
    } else {
        raiz
    };
    let k = c.mat(Mat::std(if mg { crate::maquete_magno::MESA } else { "#c6b497" }).r(0.75));
    c.cilindro(&raiz.e(v3(0.0, TOPO - 0.2, 0.0), V3::default(), 1.0), r_mesa, r_mesa, 0.4, 192, k);
    let k = c.mat(Mat::std(if mg { crate::maquete_magno::BORDA } else { "#6b4a2e" }).r(0.6));
    c.cilindro(&raiz.e(v3(0.0, TOPO - 0.42, 0.0), V3::default(), 1.0), r_mesa + 0.15, r_mesa + 0.15, 0.08, 192, k);
    if mg {
        crate::maquete_magno::manchas(&mut c, &raiz, TOPO, r_mesa);
    }
    for (i, a) in [0.3f32, 1.2, 2.1, 2.9, 3.8, 4.7, 5.6].iter().enumerate().filter(|_| !mg) {
        let r = 5.3 + (i % 2) as f32 * 0.6;
        let rr = 0.7 + (i % 3) as f32 * 0.25;
        let k = c.mat(Mat::std("#93a463").r(0.95).o(0.55));
        c.cilindro(&raiz.e(v3(a.cos() * r, TOPO + 0.005, a.sin() * r), V3::default(), 1.0), rr, rr, 0.01, 28, k);
    }
    let cofre = raiz.e(v3(0.0, TOPO, 0.0), v3(0.0, if mg { rc } else { 0.0 }, 0.0), 1.0);
    if faixa_ant.is_some() {
        // a faixa continua na mesa: sem cofre (como no video)
    } else if mg && gaveta {
        // no video o cofre nao aparece nos comparativos: na 1a cena ele encolhe e some, em vez de sumir de uma vez
        let s = if d["faixa_primeira"].as_bool().unwrap_or(false) { 1.0 - suave(t, 0.0, 0.7) } else { 0.0 };
        if s > 0.01 {
            crate::maquete_magno::cofre(&mut c, &cofre.e(V3::default(), V3::default(), s), 1.0);
        }
    } else if mg {
        // no comeco do ano a porta abre quando o mapa sai (gastou.t); nas cenas seguintes ja esta aberta
        let abre = if inicio_ano { suave(t, tg - 0.25, tg + 0.15) } else { 1.0 };
        crate::maquete_magno::cofre(&mut c, &cofre, abre);
    } else {
        let k = c.mat(Mat::std("#4b5563").r(0.4).m(0.5));
        c.caixa(&cofre.e(v3(0.0, 0.6, 0.0), V3::default(), 1.0), 1.3, 1.2, 1.2, k);
        let k = c.mat(Mat::std(&cor_do_ano).e(&cor_do_ano, 0.3).m(0.4).r(0.4));
        c.cilindro(&cofre.e(v3(0.0, 0.6, 0.61), v3(PI / 2.0, 0.0, 0.0), 1.0), 0.3, 0.3, 0.05, 28, k);
    }
    // magno: desenha no anel as pilhas que ainda nao sairam (as de quem ja foi falado nao estao mais la)
    {
        let mut no_anel: Vec<(usize, usize, usize)> = lugar_sobra.iter().zip(&sobra).map(|(k, p)| (*k, p.0, p.1)).collect();
        for (oi, (_, ps, estado)) in ordem.iter().enumerate().filter(|_| tem_anel) {
            for (j, p) in ps.iter().enumerate() {
                let fica = match *estado {
                    -2 => true,
                    -1 => false,
                    i => p_voo(i as usize, j) <= 0.0,
                };
                if fica {
                    no_anel.push((lugar[oi][j], p.0, p.1));
                }
            }
        }
        for (k, tipo, n) in no_anel {
            let (x, z) = pos0[k];
            let q = if inicio_ano { ((t - (tg + 0.05 + k as f32 * 0.9 / n0 as f32)) / 0.45).clamp(0.0, 1.0) } else { 1.0 };
            if q <= 0.0 {
                continue;
            }
            // sai pela porta do cofre e cai no lugar dela no anel
            let q = q * (2.0 - q);
            pilha_moedas(&mut c, &raiz, v3(porta.x + (x - porta.x) * q, TOPO + 0.45 * (1.0 - q) + (q * PI).sin() * 0.6, porta.z + (z - porta.z) * q), &cor_do_ano, tipo, n);
        }
    }
    for kk in (0..no_cofre).filter(|_| !mg) {
        let a = kk as f32 / (no_cofre.min(12).max(1)) as f32 * PI * 2.0 + if kk >= 12 { 0.26 } else { 0.0 };
        let r = if kk >= 12 { 1.45 } else { 1.1 };
        pilha(&mut c, &raiz, v3(a.cos() * r, TOPO, a.sin() * r), &cor_do_ano, 0.38);
    }
    if let Some(s) = saldo {
        let rotulo = s["rotulo"].as_str().unwrap_or("");
        let st = s["t"].as_f64().unwrap_or(0.0) as f32;
        if !rotulo.is_empty() && t >= st {
            let a_mais = s["tipo"].as_str() == Some("a_mais");
            let cor = if a_mais { "#d64545" } else { "#2fa36b" };
            let q0 = (s["valor"].as_f64().unwrap_or(0.0) / (pilha_mi * 1e6)).round();
            let q = (if q0 == 0.0 { 1.0 } else { q0 }).clamp(1.0, 8.0) as f32;
            let g = if mg { raiz.e(v3(0.0, TOPO, 0.0), v3(0.0, -yaw, 0.0), 1.0).e(v3(1.2, 0.0, 3.0), V3::default(), 1.0) } else { raiz.e(v3(1.9, TOPO, 0.5), V3::default(), 1.0) };
            let quantas = (q * ((t - st) / 0.8).min(1.0)).ceil() as usize;
            for kk in 0..quantas {
                pilha(&mut c, &g, v3(0.0, kk as f32 * 0.4, 0.0), cor, 0.38);
            }
            let limpo = limpar_saldo(rotulo);
            let tex = placa(&limpo, if a_mais { "A MAIS" } else { "SOBROU" }, cor);
            plano_tex(&mut c, &g.e(v3(0.0, q * 0.4 + 0.5, 0.0), v3(0.0, if mg { 0.0 } else { -yaw }, 0.0), if mg { 1.6 } else { 1.0 }), 1.5, 0.56, tex);
        }
    }
    let anos_ant = d["anos_ant"].as_object();
    for (nome, tipo) in &todos {
        let ang = ang_de(nome);
        let (x, z) = (ang.cos() * ra, ang.sin() * ra);
        let i = seto.iter().position(|s| &s.nome == nome);
        let s = i.map(|i| &seto[i]);
        let de_pe = ja.contains(&nome.as_str()) || geral;
        if s.is_none() && !de_pe {
            continue;
        }
        let f0 = i.map(|i| tps[i] * FPS).unwrap_or(-999.0);
        let nasce = if de_pe { 1.0 } else { mola(frame - f0, FPS, 14.0) };
        let valor = s.and_then(|s| s.valor).or_else(|| ja_ano.get(nome).cloned());
        let zerado = s.map(|s| s.zerado || !(s.valor.unwrap_or(0.0) > 0.0)).unwrap_or(false);
        let pilhas = match valor {
            Some(v) if v > 0.0 && (s.is_some() || ja_ano.contains_key(nome)) => ((v / (pilha_mi * 1e6)).round() as i64).clamp(1, 18) as usize,
            _ => 0,
        };
        if nasce > 0.0 {
            let g = raiz.e(v3(x, TOPO + if mg { 0.0 } else { 0.03 }, z), V3::default(), nasce.max(0.001));
            let k = c.mat(Mat::std(if zerado { "#8a8a8a" } else if mg { crate::maquete_magno::GRAMA } else { "#2e7d4f" }).r(0.7));
            if !mg {
                c.cilindro(&g.e(v3(0.0, 0.02, 0.0), V3::default(), 1.0), 0.62, 0.62, 0.04, 48, k); // magno: predio direto na mesa
            }
            let tp = s.and_then(|s| s.tipo.clone()).or_else(|| tipo.clone()).unwrap_or_else(|| "casa".into());
            if mg {
                crate::maquete_magno::casinha(&mut c, &g.e(V3::default(), v3(0.0, -ang - PI / 2.0, 0.0), 1.4), &tp, nome, zerado);
            } else {
                casinha(&mut c, &g.e(V3::default(), v3(0.0, -ang + PI / 2.0, 0.0), 1.25), &tp, zerado);
            }
        }
        let (lx, lz) = ((ang + PI / 2.0).cos(), (ang + PI / 2.0).sin());
        if let Some(oi) = ordem.iter().position(|o| &o.0 == nome) {
            for (j, &(tipo, qt)) in ordem[oi].1.iter().enumerate() {
                let p = match ordem[oi].2 {
                    _ if geral => 1.0, // visao geral: tudo no lugar
                    -1 => 1.0,
                    -2 => 0.0,
                    i => p_voo(i as usize, j),
                };
                if p <= 0.0 {
                    continue;
                }
                // fileiras de 4 ao lado do predio, uma atras da outra em direcao ao centro
                let (col, lin) = ((j % 4) as f32, (j / 4) as f32);
                let (lado, r) = (-crate::maquete_magno::FOLGA - col * 0.27, (ra - 0.1 - lin * 0.27) / ra);
                let (ax, az) = (x * r + lx * lado, z * r + lz * lado);
                let (x0, y0, z0, arco) = match lugar[oi].get(j).and_then(|k| pos0.get(*k)) {
                    Some(&(px, pz)) => (px, TOPO, pz, 0.9),
                    None => (0.0, TOPO + 1.4, 0.0, 1.4), // cena sem anel: sai de cima do cofre
                };
                pilha_moedas(&mut c, &raiz, v3(x0 + (ax - x0) * p, y0 + (TOPO - y0) * p + (p * PI).sin() * arco, z0 + (az - z0) * p), &cor_do_ano, tipo, qt);
            }
        }
        for kk in (0..pilhas).filter(|_| !mg) {
            let p = match i {
                Some(i) if !geral => p_voo(i, kk),
                _ => 1.0, // visao geral: as pilhas ja estao no lugar (nao voam de novo)
            };
            if p <= 0.0 {
                continue;
            }
            let (col, lin) = if mg { ((kk % 3) as f32, (kk / 3) as f32) } else { ((kk % 4) as f32, (kk / 4) as f32) };
            // magno: o bloco de pilhas fica AO LADO do predio (no video), nao entre a camera e ele
            let (lado, r) = if mg { (-crate::maquete_magno::FOLGA - col * 0.28, (ra - 0.1 - lin * 0.28) / ra) } else { ((col - 1.5) * 0.27, (ra - 0.95 - lin * 0.27) / ra) };
            let (ax, az) = (x * r + lx * lado, z * r + lz * lado);
            // magno: a pilha sai do lugar dela no anel do cofre (ou de cima do cofre, se o anel acabou) e voa ate o setor
            let (x0, y0, z0, arco) = match de_onde.get(&(nome.clone(), kk)) {
                Some(&(px, pz)) if mg => (px, TOPO, pz, 0.9),
                _ => (0.0, TOPO + 0.15 + if mg { 1.25 } else { 0.0 }, 0.0, 1.4),
            };
            let y1 = TOPO + if mg { 0.0 } else { 0.15 }; // magno: pilha assentada na mesa
            pilha(&mut c, &raiz, v3(x0 + (ax - x0) * p, y0 + (y1 - y0) * p + (p * PI).sin() * arco, z0 + (az - z0) * p), &cor_do_ano, 0.34 + 0.06 * (kk % 3) as f32);
        }
        if let Some(hs) = anos_ant.and_then(|o| o.get(nome)).and_then(|v| v.as_array()) {
            // magno: anos anteriores com a MESMA regra das moedas, em fileiras de 3 do outro lado do predio (um ano atras do outro)
            let mut fila = 0usize;
            for (j, h) in hs.iter().enumerate().filter(|_| mg) {
                let cor = cor_ano(h["cor"].as_str(), "#d4a537");
                let ps = compor(h["valor"].as_f64().unwrap_or(0.0), pilha_mi, j + 1);
                for (idx, &(tipo, qt)) in ps.iter().enumerate() {
                    let (col, lin) = ((idx % 3) as f32, (fila + idx / 3) as f32);
                    let rr = (ra - 0.1 - lin * 0.27) / ra;
                    let ll = crate::maquete_magno::FOLGA + col * 0.27;
                    pilha_moedas(&mut c, &raiz, v3(x * rr + lx * ll, TOPO, z * rr + lz * ll), &cor, tipo, qt);
                }
                fila += ps.len().div_ceil(3);
            }
            for (j, h) in hs.iter().enumerate().filter(|_| !mg) {
                let q = ((h["valor"].as_f64().unwrap_or(0.0) / (pilha_mi * 1e6)).round() as i64).clamp(1, 5) as usize;
                let lado = if mg { crate::maquete_magno::FOLGA } else { -0.75 - j as f32 * 0.6 };
                let r = if mg { (ra - 0.1) / ra } else { (ra - 0.95) / ra };
                let cor = cor_ano(h["cor"].as_str(), "#d4a537");
                for kk in 0..q {
                    let rr = r - ((kk / 2) as f32 + if mg { j as f32 * 3.0 } else { 0.0 }) * if mg { 0.28 } else { 0.27 } / ra;
                    let ll = if mg { lado + (kk % 2) as f32 * 0.28 } else { lado - (kk % 2) as f32 * 0.27 };
                    pilha(&mut c, &raiz, v3(x * rr + lx * ll, TOPO, z * rr + lz * ll), &cor, 0.36);
                }
            }
        }
        if let (Some(s), Some(ii)) = (s, i) {
            if !zerado && !geral && ii as isize == i_ativo {
                let pl = if mg { crate::maquete_magno::placa } else { placa };
                let mut texs = vec![pl(&s.rotulo, &ano, &cor_do_ano)];
                for a in s.anteriores.iter().take(2) {
                    texs.push(pl(&a.1, &a.0, &cor_ano(a.2.as_deref(), "#d4a537")));
                }
                for (j, tex) in texs.into_iter().enumerate() {
                    let pop = mola(frame - f0 - 14.0 - j as f32 * 8.0, FPS, 12.0);
                    if pop <= 0.0 {
                        continue;
                    }
                    let esc = pop * if j == 0 { 1.0 } else { 0.8 };
                    if mg {
                        // magno: plaquinha menor, acima das pilhas (ao lado do predio), sem tampar a casa
                        let (r, lado) = ((ra - 0.3) / ra, -crate::maquete_magno::FOLGA - 0.28);
                        // em cima do predio (as pilhas agora sao altas e ficam ao lado): nao tampa a casa nem encosta no titulo
                        let _ = (r, lado);
                        plano_tex(&mut c, &raiz.e(v3(x, TOPO + 1.62 + j as f32 * 0.45, z), v3(0.0, -yaw, 0.0), esc), 1.15, 0.43, tex);
                    } else {
                        plano_tex(&mut c, &raiz.e(v3(x * 0.82, TOPO + 1.35 + j as f32 * 0.52, z * 0.82), v3(0.0, -yaw, 0.0), esc), 1.5, 0.56, tex);
                    }
                }
            }
        }
    }
    if gaveta && mg {
        let g = raiz.e(v3(0.0, TOPO, 0.0), v3(0.0, psi - yaw, 0.0), 1.0);
        montar_gaveta_magno(&mut c, &g, d, frame);
    } else if gaveta {
        let g = raiz.e(v3(0.0, TOPO, 0.0), v3(0.0, -yaw, 0.0), 1.0);
        montar_gaveta(&mut c, &g, d, frame);
    } else if let Some(fa) = faixa_ant {
        let g = raiz.e(v3(0.0, TOPO, 0.0), v3(0.0, psi - yaw, 0.0), 1.0);
        montar_gaveta_magno(&mut c, &g, fa, 1e6);
    }
    if mg {
        c.sombra = Some(crate::maquete_magno::luz(&raiz, rc));
    }
    c
}

// "GASTOU R$ 190 MI A MAIS" -> "R$ 190 MI"
fn limpar_saldo(r: &str) -> String {
    let mut s = r.trim().to_string();
    for p in ["GASTOU ", "SOBRARAM ", "SOBROU "] {
        if s.to_uppercase().starts_with(p) {
            s = s[p.len()..].trim_start().to_string();
            break;
        }
    }
    if s.to_uppercase().ends_with(" A MAIS") {
        s.truncate(s.len() - " A MAIS".len());
    }
    s.trim_end().to_string()
}

// --- comparativo no visual magno: faixa comprida, colunas altas que se acumulam, camera acompanhando ---
const PASSO_MG: f32 = 0.96; // colunas mais juntas (17/11, pedido do Carlos); o bloco tem ~0,82 de largura

/// no video a faixa fica do outro lado do cofre (a camera passa por cima dele olhando para fora)

struct Faixa {
    valores: Vec<f64>,
    rotulos: Vec<String>,
    cores: Vec<String>,
    t: Vec<f32>, // segundos da cena em que a coluna nasce (as herdadas de cenas anteriores vem negativas)
    max: f64,
}

fn faixa_magno(d: &Value) -> Faixa {
    let valores: Vec<f64> = d["valores"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
    let rotulos: Vec<String> = d["rotulos"].as_array().map(|a| a.iter().map(|v| v.as_str().map(String::from).unwrap_or(v.to_string())).collect()).unwrap_or_default();
    let cores: Vec<String> = d["cores"].as_array().map(|a| a.iter().map(|v| cor_ano(v.as_str(), "#d4a537")).collect()).unwrap_or_default();
    let t: Vec<f32> = (0..valores.len()).map(|i| d["barras"][i]["t"].as_f64().map(|x| x as f32).unwrap_or(0.3 + i as f32 * 0.8)).collect();
    let max = d["max_faixa"].as_f64().unwrap_or_else(|| valores.iter().cloned().fold(1.0, f64::max));
    Faixa { valores, rotulos, cores, t, max }
}


/// (inclinacao, distancia, sobe) da camera da faixa; MOTOR_FAIXA="p,d,s" afina comparando com o video
fn cam_faixa() -> (f32, f32, f32) {
    static C: OnceLock<(f32, f32, f32)> = OnceLock::new();
    *C.get_or_init(|| {
        let v: Vec<f32> = std::env::var("MOTOR_FAIXA").ok().map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default();
        if v.len() == 3 { (v[0], v[1], v[2]) } else { (0.85, 17.0, 0.0) } // mais longe: as colunas agora sao altas
    })
}

/// colunas que a faixa tera no fim (todas as cenas de comparativo): fixa o lugar de cada coluna na mesa
fn total_faixa(d: &Value, n: usize) -> usize {
    d["faixa_total"].as_u64().map(|x| x as usize).unwrap_or(n).max(n)
}

/// A faixa e RETA: comeca colada na frente de um predio da esquerda e atravessa a mesa pelo meio (como no video).
/// O corredor em frente ao predio e livre (as pilhas dele ficam dos lados) e o cofre some nessas cenas.
/// shortcut: com mais de ~13 colunas a ponta direita chega nos predios do outro lado; ai diminuir PASSO_MG
fn x0_faixa() -> f32 {
    -(crate::maquete_magno::ANEL - 1.15)
}

/// x da coluna i: sempre o mesmo, em sequencia da esquerda para a direita (nao depende de quantas ja nasceram)
fn x_col(i: usize) -> f32 {
    x0_faixa() + 0.6 + PASSO_MG * (i as f32 + 0.5)
}

/// (deslocamento, inclinacao) da camera da faixa; escala 1. A camera desliza de lado, de coluna em coluna
fn faixa_cam(d: &Value, frame: f32) -> (V3, f32) {
    let fx = faixa_magno(d);
    let n = fx.valores.len().max(1);
    let t = frame / FPS;
    // foco: anda de coluna em coluna conforme nascem, sempre mostrando a anterior junto
    let mut foco = x_col(0);
    for i in 1..n {
        foco += PASSO_MG * suave(t, fx.t[i] - 0.4, fx.t[i] + 0.6);
    }
    foco -= PASSO_MG * 0.5;
    let (p, dist, sobe) = cam_faixa();
    let f = v3(foco, TOPO + 0.55, 0.0);
    let (sn, cs) = p.sin_cos();
    let (py, pz) = (cs * f.y - sn * f.z, sn * f.y + cs * f.z);
    // a faixa fica alinhada com a tela: so desloca para o foco ir ao centro
    (v3(-f.x, -py + sobe, (13.0 - dist) - pz), p)
}

fn montar_gaveta_magno(c: &mut Cena3, pai: &M4, d: &Value, frame: f32) {
    let fx = faixa_magno(d);
    let nt = total_faixa(d, fx.valores.len().max(1));
    let largura = nt as f32 * PASSO_MG + 1.2;
    let x0 = x0_faixa();
    // 1a cena: a faixa se desenrola a partir do predio (nao aparece de uma vez)
    let entra = if d["faixa_primeira"].as_bool().unwrap_or(false) { suave(frame / FPS, 0.1, 1.3).max(0.001) } else { 1.0 };
    let k = c.mat(Mat::std("#1e5130").r(0.85));
    c.caixa(&pai.e(v3(x0 + largura * entra / 2.0, 0.015, 0.0), V3::default(), 1.0), largura * entra, 0.03, 1.9, k);
    if entra >= 0.999 {
        let rotulo = d["faixa_magno"].as_str().or(d["titulo"].as_str()).unwrap_or("");
        // fundo claro na beirada inteira e a frase por cima, em pedacos de 2,56 (1024 px a 0,24 de altura), sem esticar
        let lado = 0.24 * 1024.0 / 96.0;
        let n = faixa_titulo_partes(rotulo).min((largura / lado) as usize);
        for k in 0..n {
            plano_tex(c, &pai.e(v3(x0 + lado * (k as f32 + 0.5), 0.035, 0.82), v3(-PI / 2.0, 0.0, 0.0), 1.0), lado, 0.24, faixa_titulo_parte(rotulo, k));
        }
        // o resto da beirada so com o fundo (lado a lado com a frase, nunca por cima: duas camadas juntas se cobriam)
        let resto = largura - n as f32 * lado;
        if resto > 0.01 {
            plano_tex(c, &pai.e(v3(x0 + n as f32 * lado + resto / 2.0, 0.035, 0.82), v3(-PI / 2.0, 0.0, 0.0), 1.0), resto, 0.24, faixa_titulo_parte("", 0));
        }
    }
    for i in 0..fx.valores.len() {
        let f0 = fx.t[i] * FPS;
        let nasce = mola(frame - f0, FPS, 14.0);
        // mesmas tres moedas, na escala da faixa (legenda "AQUI, MOEDAS: ..."); fileiras de 3, bem juntinhas
        let un = d["faixa_unidade"].as_f64().unwrap_or(1.0);
        let ps = crate::maquete_magno::compor(fx.valores[i] * un, crate::maquete_magno::pilha_faixa_mi(fx.max * un), i);
        let visiveis = (nasce.clamp(0.0, 1.0) * ps.len() as f32).ceil() as usize;
        let cor = fx.cores.get(i).cloned().unwrap_or_else(|| "#d4a537".into());
        let col_g = pai.e(v3(x_col(i), 0.03, -0.15), V3::default(), 1.0);
        let filas = ps.len().div_ceil(3) as f32;
        let mut topo = 0.0f32;
        for (idx, &(tipo, qt)) in ps.iter().take(visiveis).enumerate() {
            let (col, lin) = ((idx % 3) as f32, (idx / 3) as f32);
            pilha_moedas(c, &col_g, v3((col - 1.0) * 0.27, 0.0, (lin - (filas - 1.0) / 2.0) * 0.27), &cor, tipo, qt);
            topo = topo.max(crate::maquete_magno::alt(tipo, qt));
        }
        if nasce > 0.0 {
            let rot = fx.rotulos.get(i).cloned().unwrap_or_else(|| crate::desenho::br(fx.valores[i], 0));
            let (l1, l2) = match rot.find("R$") {
                Some(k) => (rot[k..].trim().to_string(), rot[..k].trim().to_string()),
                None => (rot.clone(), String::new()),
            };
            let y = topo + 0.45 + (i % 2) as f32 * 0.35;
            plano_tex(c, &col_g.e(v3(0.0, y, 0.0), V3::default(), nasce), 1.0, 0.37, crate::maquete_magno::placa(&l1, &l2, &cor));
        }
    }
}

fn montar_gaveta(c: &mut Cena3, pai: &M4, d: &Value, frame: f32) {
    let valores: Vec<f64> = d["valores"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_else(|| vec![1.0, 2.0]);
    let rotulos: Vec<String> = d["rotulos"].as_array().map(|a| a.iter().map(|v| v.as_str().map(String::from).unwrap_or(v.to_string())).collect()).unwrap_or_default();
    let cor_padrao = cor_ano(d["cor"].as_str(), "#d4a537");
    let cores: Vec<String> = d["cores"].as_array().map(|a| a.iter().map(|v| cor_ano(v.as_str(), "#d4a537")).collect()).unwrap_or_default();
    let ts: Vec<Option<f32>> = d["barras"].as_array().map(|a| a.iter().map(|b| b["t"].as_f64().map(|x| x as f32)).collect()).unwrap_or_default();
    let max = valores.iter().cloned().fold(1.0, f64::max);
    let n = valores.len().max(1);
    let largura = (1.0 * n as f32 + 0.6).min(5.2);
    let passo = largura / n as f32;
    let titulo = d["titulo"].as_str().unwrap_or("");
    let g = pai.e(v3(0.0, 1.27, 1.3), V3::default(), if n > 3 { 0.95 } else { 1.2 });
    let lw = (largura + 0.6).max(4.4);
    let k = c.mat(Mat::std("#2e7d4f").r(0.8));
    c.caixa(&g.e(v3(0.0, -1.29, 0.2), V3::default(), 1.0), lw, 0.03, 2.0, k);
    plano_tex(c, &g.e(v3(0.0, -1.27, 1.21), v3(-PI / 2.0, 0.0, 0.0), 1.0), lw, 0.44, faixa_titulo(titulo));
    for (i, v) in valores.iter().enumerate() {
        let f0 = ts.get(i).cloned().flatten().unwrap_or(0.3 + i as f32 * 0.8) * FPS;
        let nasce = mola(frame - f0, FPS, 14.0);
        let qtd = ((v / max * 12.0).round() as usize).max(1);
        let x = -largura / 2.0 + passo / 2.0 + i as f32 * passo;
        let cor = cores.get(i).cloned().unwrap_or_else(|| cor_padrao.clone());
        let col_g = g.e(v3(x, -1.27, 0.1), V3::default(), 1.0);
        for kk in 0..qtd {
            let alto = (nasce * qtd as f32 - kk as f32).clamp(0.0, 1.0);
            if alto <= 0.0 {
                continue;
            }
            let (col, lin) = ((kk % 3) as f32, (kk / 3) as f32);
            pilha(c, &col_g, v3((col - 1.0) * 0.29, 0.0, (lin - 1.5) * 0.29), &cor, 0.42 * alto + 0.001);
        }
        if nasce > 0.0 {
            let pw = (passo * 1.05).max(1.45).min(1.9);
            let rot = rotulos.get(i).cloned().unwrap_or_else(|| crate::desenho::br(*v, 0));
            let y = 1.15 + if n > 3 { (i % 2) as f32 * 0.62 } else { 0.0 };
            plano_tex(c, &col_g.e(v3(0.0, y, 0.0), V3::default(), nasce), pw, pw * 0.37, placa(&rot, "", &cor));
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn saldo_e_cores() {
        assert_eq!(limpar_saldo("GASTOU R$ 190 MI A MAIS"), "R$ 190 MI");
        assert_eq!(limpar_saldo("SOBROU R$ 5 MI"), "R$ 5 MI");
        assert_eq!(cor_ano(Some("rosa claro"), "#000000"), "#efb3bd");
        assert_eq!(cor_ano(Some("#123abc"), "#000000"), "#123abc");
        assert_eq!(cor_ano(None, "#d4a537"), "#d4a537");
    }
}
