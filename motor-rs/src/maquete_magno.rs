// Visual "magno" da mesa: recriado OLHANDO o video modelo do Salvador do Magno (cores medidas nos quadros, formas descritas a olho).
// Nenhum codigo, guia ou instrucao do pacote do Magno entra aqui (licenca pessoal do estudio dele).
use crate::desenho::{ret, Cor};
use crate::maquete::{com_cache, fontes, plano_tex};
use crate::tres::{v3, Cena3, Mat, Tex, M4, V3};
use std::f32::consts::PI;
use std::sync::Arc;

// albedos calibrados para, depois da luz e do ACES, darem as cores medidas no video (mesa #957f61, grama #316830)
pub const MESA: &str = "#b39a74";
pub const BORDA: &str = "#3a2a1c";
pub const GRAMA: &str = "#2f7a34";
pub const COFRE: &str = "#45413d";
/// raio do anel dos setores e da mesa no visual magno (13 setores: ~3,4 de arco cada, cabe predio + 3 colunas de pilhas)
pub const ANEL: f32 = 8.3;
pub const MESA_R: f32 = 10.2;
/// distancia lateral do centro do predio ate a 1a coluna de pilhas (meia largura do maior predio + raio da pilha + folga)
pub const FOLGA: f32 = 1.2;

/// manchas de grama grandes na beira da mesa (opacas, como no video)
pub fn manchas(c: &mut Cena3, raiz: &M4, topo: f32, r_mesa: f32) {
    for (i, a) in [0.35f32, 1.25, 2.2, 3.05, 3.9, 4.75, 5.6].iter().enumerate() {
        // um pouco maiores, mas sempre inteiros dentro da mesa (antes passavam da borda e ficavam soltos no ar)
        let rr = 1.25 + (i % 3) as f32 * 0.3;
        let r = (r_mesa - 0.6 + (i % 2) as f32 * 0.4).min(r_mesa - rr - 0.15);
        let k = c.mat(Mat::std(GRAMA).r(0.95));
        c.cilindro(&raiz.e(v3(a.cos() * r, topo + 0.006, a.sin() * r), V3::default(), 1.0), rr, rr, 0.012, 40, k);
    }
}

/// cofre: caixa escura fosca com a fechadura redonda preta na frente
/// abre: 0 = porta fechada, 1 = aberta (gira na dobradica da direita, para fora)
pub fn cofre(c: &mut Cena3, m: &M4, abre: f32) {
    let k = c.mat(Mat::std(COFRE).r(0.7).m(0.1));
    c.caixa_r(&m.e(v3(0.0, 0.6, 0.0), V3::default(), 1.0), 1.3, 1.2, 1.2, 0.09, k);
    // boca escura com o dourado do dinheiro la dentro
    let k = c.mat(Mat::std("#050506").r(0.9));
    c.cilindro(&m.e(v3(0.0, 0.62, 0.603), v3(PI / 2.0, 0.0, 0.0), 1.0), 0.25, 0.25, 0.01, 32, k);
    let k = c.mat(Mat::std("#b8902e").e("#d4a537", 0.15).m(0.5).r(0.4));
    c.cilindro(&m.e(v3(0.0, 0.5, 0.5), V3::default(), 1.0), 0.16, 0.16, 0.12, 20, k);
    let porta = m.e(v3(0.27, 0.62, 0.62), v3(0.0, abre * 1.9, 0.0), 1.0);
    let k = c.mat(Mat::std("#5b5852").r(0.35).m(0.6));
    c.cilindro(&porta.e(v3(-0.27, 0.0, 0.02), v3(PI / 2.0, 0.0, 0.0), 1.0), 0.26, 0.26, 0.05, 32, k);
    let k = c.mat(Mat::std("#0d0d0f").r(0.5));
    c.cilindro(&porta.e(v3(-0.27, 0.0, 0.05), v3(PI / 2.0, 0.0, 0.0), 1.0), 0.11, 0.11, 0.03, 24, k);
    let k = c.mat(Mat::std("#8a857c").r(0.3).m(0.7));
    c.caixa(&porta.e(v3(-0.27, 0.0, 0.07), V3::default(), 1.0), 0.16, 0.025, 0.02, k);
}

/// REGRA DAS MOEDAS (magno, 17/11/2026): tres tamanhos de moeda, cada um com valor fixo no video inteiro.
/// GRANDE = pilha_mi / 40 (R$ 5 mi quando o roteiro traz pilha_mi 200), MEDIA = grande / 5, PEQUENA = grande / 20.
/// (17/11: o Carlos achou pouca moeda com a grande a R$ 20 mi; agora sao 4x mais moedas e pilhas ~3x mais altas)
/// Valor pequeno aparece como poucas moedas pequenas; valor grande, como varias pilhas de moedas grandes.
/// (raio, espessura, quantas PEQUENAS vale)
pub const MOEDA: [(f32, f32, usize); 3] = [(0.13, 0.042, 20), (0.1, 0.036, 4), (0.075, 0.03, 1)];
/// alturas das pilhas de moedas grandes, em rodizio: o cofre nao fica com todas as pilhas iguais
const ALTURAS: [usize; 8] = [36, 24, 44, 30, 40, 22, 38, 28];

/// valor em reais da moeda de um tamanho (0 grande, 1 media, 2 pequena)
pub fn valor_moeda(tipo: usize, pilha_mi: f64) -> f64 {
    pilha_mi * 1e6 / 40.0 / 20.0 * MOEDA[tipo].2 as f64
}

/// altura de uma pilha com n moedas de um tamanho
pub fn alt(tipo: usize, n: usize) -> f32 {
    n as f32 * MOEDA[tipo].1
}

/// As pilhas (tamanho da moeda, quantas moedas) que mostram um valor em reais. Valor maior que zero tem sempre
/// pelo menos 1 moeda pequena. `semente` so muda por onde comeca o rodizio de alturas (cada setor diferente do outro).
/// shortcut: acima de 8000 moedas grandes (R$ 40 bi com pilha de 200 mi) corta; cidade maior pede pilha_mi maior no roteiro
pub fn compor(valor: f64, pilha_mi: f64, semente: usize) -> Vec<(usize, usize)> {
    if valor <= 0.0 || pilha_mi <= 0.0 {
        return Vec::new();
    }
    let u = ((valor / valor_moeda(2, pilha_mi)).round() as usize).max(1);
    let (mut g, m, p) = ((u / 20).min(8000), (u % 20) / 4, u % 4);
    let mut r = Vec::new();
    let mut k = semente;
    while g > 0 {
        let mut n = ALTURAS[k % ALTURAS.len()].min(g);
        if g - n < 3 {
            n = g; // sobra de 1 ou 2 moedas entra nesta pilha (nada de pilha grande com 1 moeda so)
        }
        r.push((0, n));
        g -= n;
        k += 1;
    }
    if m > 0 {
        r.push((1, m));
    }
    if p > 0 {
        r.push((2, p));
    }
    r
}

/// "R$ 20 MI", "R$ 500 MIL": valor de moeda para a legenda
pub fn rotulo_moeda(v: f64) -> String {
    let (n, un) = if v >= 1e9 { (v / 1e9, "BI") } else if v >= 1e6 { (v / 1e6, "MI") } else { (v / 1e3, "MIL") };
    let t = format!("{n:.2}");
    format!("{} {un}", t.trim_end_matches('0').trim_end_matches('.').replace('.', ","))
}

/// legenda das tres moedas: "MOEDAS: R$ 20 MI · 5 MI · 1 MI"
pub fn legenda_moedas(pilha_mi: f64) -> String {
    format!("MOEDAS: R$ {} · {} · {}", rotulo_moeda(valor_moeda(0, pilha_mi)), rotulo_moeda(valor_moeda(1, pilha_mi)), rotulo_moeda(valor_moeda(2, pilha_mi)))
}

/// Escala da faixa do comparativo (os valores la sao bem menores que os dos setores): devolve o `pilha_mi` da faixa.
/// A moeda grande vale 2, 20, 200... milhoes, a menor escala em que a maior coluna fica com ate ~130 moedas grandes
pub fn pilha_faixa_mi(maior: f64) -> f64 {
    let n = ((maior / 130.0 / 2e6).max(1e-12).log10().ceil()).max(-6.0);
    2.0 * 10f64.powf(n) * 10.0
}

/// Em que unidade esta um valor da faixa (1, mil, milhao ou bilhao de reais), lendo o proprio rotulo dele:
/// valor 79,99 com rotulo "2024 PREVIA R$ 80 MI" esta em milhoes. Sem rotulo legivel: reais
pub fn unidade_do_rotulo(valor: f64, rotulo: &str) -> f64 {
    let Some(k) = rotulo.find("R$") else { return 1.0 };
    let mut partes = rotulo[k + 2..].split_whitespace();
    let Some(n) = partes.next().and_then(|x| x.replace('.', "").replace(',', ".").parse::<f64>().ok()) else { return 1.0 };
    let reais = n * match partes.next().map(|x| x.to_uppercase()).as_deref() {
        Some("BI") => 1e9,
        Some("MI") => 1e6,
        Some("MIL") => 1e3,
        _ => 1.0,
    };
    if valor <= 0.0 || reais <= 0.0 {
        return 1.0;
    }
    10f64.powf(((reais / valor).log10() / 3.0).round() * 3.0)
}

#[cfg(test)]
mod teste_moedas {
    use super::*;

    #[test]
    fn unidade_vem_do_rotulo() {
        assert_eq!(unidade_do_rotulo(79.99, "2024 PREVIA R$ 80 MI"), 1e6);
        assert_eq!(unidade_do_rotulo(233_170_000.0, "EVENTOS R$ 233 MI"), 1.0);
        assert_eq!(unidade_do_rotulo(2.69, "R$ 2,69 BI"), 1e9);
        assert_eq!(unidade_do_rotulo(457.11, "R$ 457 MIL"), 1e3);
        assert_eq!(unidade_do_rotulo(10.0, "sem valor"), 1.0);
    }

    fn reais(ps: &[(usize, usize)], pilha_mi: f64) -> f64 {
        ps.iter().map(|(t, n)| *n as f64 * valor_moeda(*t, pilha_mi)).sum()
    }

    #[test]
    fn tres_moedas_de_valor_fixo() {
        assert_eq!((valor_moeda(0, 200.0), valor_moeda(1, 200.0), valor_moeda(2, 200.0)), (5e6, 1e6, 0.25e6));
        assert_eq!(compor(457_110.0, 200.0, 0), vec![(2, 2)]); // R$ 457 mil: 2 moedas pequenas, nao uma pilha
        assert_eq!(compor(91_183_546.0, 200.0, 0), vec![(0, 18), (1, 1), (2, 1)]); // 18 x 5 + 1 + 0,25
        assert_eq!(compor(8e6, 200.0, 0), vec![(0, 1), (1, 3)]);
        assert!(compor(0.0, 200.0, 0).is_empty());
        assert_eq!(legenda_moedas(200.0), "MOEDAS: R$ 5 MI · 1 MI · 250 MIL");
        assert_eq!(legenda_moedas(20.0), "MOEDAS: R$ 500 MIL · 100 MIL · 25 MIL");
    }

    #[test]
    fn as_moedas_somam_o_valor_e_valor_maior_nunca_mostra_menos() {
        let v = [0.45e6, 3e6, 50e6, 91e6, 220e6, 309e6, 1.84e9, 2.69e9, 3.0e9, 12.4e9];
        for (k, x) in v.iter().enumerate() {
            let ps = compor(*x, 200.0, k);
            assert!((reais(&ps, 200.0) - x).abs() <= 0.6e6, "{x}: as moedas somam {}", reais(&ps, 200.0));
            assert!(ps.iter().filter(|p| p.0 == 0).all(|p| p.1 >= 3 || ps.iter().filter(|q| q.0 == 0).count() == 1), "pilha grande com 1 ou 2 moedas: {ps:?}");
            assert!(ps.iter().all(|p| p.1 <= 46), "pilha alta demais: {ps:?}");
        }
        assert!(v.windows(2).all(|w| reais(&compor(w[0], 200.0, 0), 200.0) <= reais(&compor(w[1], 200.0, 0), 200.0)));
        // o cofre de Salvador nao tem todas as pilhas iguais
        let alturas: std::collections::HashSet<usize> = compor(12.4e9, 200.0, 0).iter().map(|p| p.1).collect();
        assert!(alturas.len() >= 6);
    }

    #[test]
    fn escala_da_faixa() {
        assert_eq!(pilha_faixa_mi(233e6), 20.0); // moeda grande de R$ 500 mil: 233 mi = 466 grandes
        assert_eq!(pilha_faixa_mi(2.7e9), 2000.0);
        assert!(compor(233.17e6, pilha_faixa_mi(233.17e6), 0).len() <= 18); // 6 fileiras de 3 cabem na faixa (1,9 de fundo)
    }
}

/// pilhas do cofre em aneis certinhos em volta dele (como no video): do anel de dentro para fora, igualmente espacadas
pub fn pos_cofre(k: usize, total: usize) -> (f32, f32) {
    let (mut resto, mut anel) = (k, 0usize);
    loop {
        let r = 1.4 + anel as f32 * 0.42;
        let cabe = ((2.0 * PI * r) / 0.4).floor() as usize;
        let ja: usize = (0..anel).map(|a| ((2.0 * PI * (1.4 + a as f32 * 0.42)) / 0.4).floor() as usize).sum();
        let neste = cabe.min(total.saturating_sub(ja)).max(1);
        if resto < cabe {
            let a = resto as f32 / neste as f32 * 2.0 * PI + anel as f32 * 0.21;
            return (a.cos() * r, a.sin() * r);
        }
        resto -= cabe;
        anel += 1;
    }
}

#[cfg(test)]
mod testes {
    #[test]
    fn pilhas_do_cofre_nao_se_encostam_nem_entram_no_cofre() {
        let ps: Vec<(f32, f32)> = (0..80).map(|k| super::pos_cofre(k, 80)).collect();
        for (i, a) in ps.iter().enumerate() {
            assert!((a.0 * a.0 + a.1 * a.1).sqrt() > 1.2, "dentro do cofre");
            for b in &ps[i + 1..] {
                assert!(((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() > 0.38, "pilhas encostadas");
            }
        }
    }
}

/// luz que projeta sombra: vem de tras e da direita da camera (no video as sombras caem para a frente e para a esquerda),
/// presa ao tampo da mesa; a forca compensa o angulo para o tampo iluminado manter a cor medida. MOTOR_LUZ=cima,direita,frente ajusta
/// luz que projeta sombra: vem do lado da camera, do alto e da esquerda, baixa (~21 graus acima da mesa), entao a
/// sombra sai longa PARA TRAS e para a direita dos objetos, nunca na frente deles. MOTOR_LUZ=cima,direita,frente
pub fn luz(raiz: &M4, rc: f32) -> V3 {
    // a direcao fica presa na MESA (nao na camera): quando a camera gira, a sombra fica parada no lugar.
    // rc = giro do cofre (porta de frente para a camera da abertura do ano): "direita" e "para a camera" daquele momento
    let k: Vec<f32> = std::env::var("MOTOR_LUZ").ok().map(|v| v.split(',').filter_map(|x| x.parse().ok()).collect()).unwrap_or_default();
    let (cu, cr, cf) = if k.len() == 3 { (k[0], k[1], k[2]) } else { (0.636, -0.9, 0.25) }; // ~34 graus: sombra 45% menor que a de 0.35 (comprimento ~ 1/tan)
    let u = raiz.d(v3(0.0, 1.0, 0.0)).norm();
    let l = raiz.d(M4::ry(rc).d(v3(cr, cu, cf))).norm();
    let l0 = v3(4.0, 6.0, 8.0).norm();
    l * (1.8 * u.dot(l0) / u.dot(l).max(0.2)).min(3.2)
}


/// placa verde do nome na fachada (fundo verde escuro, letra verde clara, como no video)
fn placa_nome(nome: &str) -> Arc<Tex> {
    com_cache(format!("magno-nome|{nome}"), || {
        let mut pm = tiny_skia::Pixmap::new(320, 64).unwrap();
        ret(&mut pm, 0.0, 0.0, 320.0, 64.0, 8.0, Cor::hex("#14301f"));
        ret(&mut pm, 4.0, 4.0, 312.0, 56.0, 6.0, Cor::hex("#1b3d28"));
        let f = &fontes().arial_black;
        let t = nome.to_uppercase();
        let mut px = 34.0;
        while f.largura(px, &t, 1.0) > 290.0 && px > 12.0 {
            px -= 2.0;
        }
        let w = f.largura(px, &t, 1.0);
        f.desenhar(&mut pm, px, 160.0 - w / 2.0, 32.0 + px * 0.36, &t, 1.0, Cor::hex("#3fd17a"));
        pm
    })
}

/// setor sem fachada (arvores, campo, onibus): placa verde do nome em dois postes, na frente, virada para a camera
fn placa_em_pe(c: &mut Cena3, m: &M4, nome: &str) {
    let k = c.mat(Mat::std("#2b2b2e").r(0.7));
    for x in [-0.3f32, 0.3] {
        c.cilindro(&m.e(v3(x, 0.17, 0.62), V3::default(), 1.0), 0.018, 0.018, 0.34, 10, k);
    }
    plano_tex(c, &m.e(v3(0.0, 0.36, 0.635), V3::default(), 1.0), 0.72, 0.14, placa_nome(&nome_placa(nome)));
}

/// nome que vai na placa do predio (no video, a Administracao aparece como PREFEITURA)
fn nome_placa(nome: &str) -> String {
    if nome.to_lowercase().contains("administra") {
        return "Prefeitura".into();
    }
    if nome.to_lowercase().starts_with("meio ambiente") {
        return "Meio Ambiente".into();
    }
    nome.split([' ', ',']).next().unwrap_or(nome).to_string()
}

fn campo_futebol() -> Arc<Tex> {
    com_cache("magno-campo".into(), || {
        let mut pm = tiny_skia::Pixmap::new(320, 200).unwrap();
        ret(&mut pm, 0.0, 0.0, 320.0, 200.0, 0.0, Cor::hex("#2e8b3e"));
        let b = Cor(1.0, 1.0, 1.0, 0.95);
        let l = |pm: &mut tiny_skia::Pixmap, x: f32, y: f32, w: f32, h: f32| ret(pm, x, y, w, h, 0.0, b);
        l(&mut pm, 10.0, 10.0, 300.0, 4.0);
        l(&mut pm, 10.0, 186.0, 300.0, 4.0);
        l(&mut pm, 10.0, 10.0, 4.0, 180.0);
        l(&mut pm, 306.0, 10.0, 4.0, 180.0);
        l(&mut pm, 158.0, 10.0, 4.0, 180.0);
        l(&mut pm, 10.0, 60.0, 40.0, 4.0);
        l(&mut pm, 10.0, 136.0, 40.0, 4.0);
        l(&mut pm, 50.0, 60.0, 4.0, 80.0);
        l(&mut pm, 270.0, 60.0, 40.0, 4.0);
        l(&mut pm, 270.0, 136.0, 40.0, 4.0);
        l(&mut pm, 266.0, 60.0, 4.0, 80.0);
        if let Some(p) = tiny_skia::PathBuilder::from_circle(160.0, 100.0, 28.0) {
            let st = tiny_skia::Stroke { width: 4.0, ..Default::default() };
            pm.stroke_path(&p, &b.paint(), &st, tiny_skia::Transform::identity(), None);
        }
        pm
    })
}

/// predio do setor, escolhido pelo nome falado (como aparece no video): cada um tem uma forma, com acabamento
/// (alicerce, janelas com moldura, porta, placa do nome, telhado com beiral)
pub fn casinha(c: &mut Cena3, m: &M4, tipo: &str, nome: &str, zerado: bool) {
    let caixa = |c: &mut Cena3, pos: V3, w: f32, h: f32, d: f32, mat: Mat| {
        let k = c.mat(mat);
        // so as pecas grandes ganham quina arredondada (as pequenas nao aparecem e custariam triangulos)
        let menor = w.min(h).min(d);
        c.caixa_r(&m.e(pos, V3::default(), 1.0), w, h, d, if menor >= 0.1 { (menor * 0.15).min(0.04) } else { 0.0 }, k);
    };
    let cinza = |s: &'static str| if zerado { "#6f6f6f" } else { s };
    let n = nome.to_lowercase();
    if n.contains("agricultura") {
        // trator verde (como no video): capo, cabine com vidro escuro, rodas traseiras grandes e escapamento
        let verde = cinza("#2f7a34");
        caixa(c, v3(0.12, 0.2, 0.0), 0.5, 0.16, 0.3, Mat::std(verde).r(0.5));
        caixa(c, v3(0.2, 0.3, 0.0), 0.34, 0.06, 0.26, Mat::std(verde).r(0.5));
        caixa(c, v3(-0.18, 0.36, 0.0), 0.28, 0.3, 0.32, Mat::std(cinza("#8f9294")).r(0.45));
        caixa(c, v3(-0.18, 0.4, 0.0), 0.29, 0.14, 0.33, Mat::std("#15171a").r(0.2));
        caixa(c, v3(-0.18, 0.525, 0.0), 0.32, 0.03, 0.36, Mat::std(verde).r(0.5));
        let k = c.mat(Mat::std("#3a3a3c").r(0.4).m(0.6));
        c.cilindro(&m.e(v3(0.3, 0.45, 0.08), V3::default(), 1.0), 0.015, 0.015, 0.18, 8, k);
        let k = c.mat(Mat::std("#111214").r(0.85));
        let aro = c.mat(Mat::std("#d9b23a").r(0.5));
        for (x, r, w) in [(-0.2f32, 0.17f32, 0.09f32), (0.3, 0.1, 0.06)] {
            for z in [-0.2f32, 0.2] {
                c.cilindro(&m.e(v3(x, r, z), v3(PI / 2.0, 0.0, 0.0), 1.0), r, r, w, 20, k);
                c.cilindro(&m.e(v3(x, r, z * 1.06), v3(PI / 2.0, 0.0, 0.0), 1.0), r * 0.5, r * 0.5, w * 1.02, 14, aro);
            }
        }
        placa_em_pe(c, m, nome);
        return;
    }
    if n.contains("meio ambiente") && !zerado {
        // arvores altas: tronco grosso escuro e copa redonda preta
        for (x, z, r, h) in [(-0.38f32, -0.1f32, 0.36f32, 0.8f32), (0.22, 0.18, 0.3, 0.68), (0.52, -0.28, 0.27, 0.6)] {
            let k = c.mat(Mat::std("#1c1613").r(0.9));
            c.cilindro(&m.e(v3(x, h / 2.0, z), V3::default(), 1.0), 0.045, 0.065, h, 20, k);
            let k = c.mat(Mat::std(cinza("#141210")).r(0.85));
            c.esfera(&m.e(v3(x, h + r * 0.55, z), V3::default(), 1.0), r, 28, k);
        }
        placa_em_pe(c, m, nome);
        return;
    }
    if tipo == "campo" || n.contains("esporte") {
        let k = c.mat(Mat::basico(campo_futebol()));
        c.plano(&m.e(v3(0.0, 0.03, 0.0), v3(-PI / 2.0, 0.0, 0.0), 1.0), 1.5, 0.95, k);
        for x in [-0.72f32, 0.72] {
            for z in [-0.12f32, 0.12] {
                caixa(c, v3(x, 0.12, z), 0.025, 0.2, 0.025, Mat::std("#f2f2f2").r(0.4));
            }
            caixa(c, v3(x, 0.225, 0.0), 0.025, 0.025, 0.27, Mat::std("#f2f2f2").r(0.4));
        }
        // arquibancada atras da quadra: 4 degraus cinza com assento claro e um guarda-corpo no alto
        for i in 0..4 {
            let (h, z) = (0.07 + i as f32 * 0.07, -0.6 - i as f32 * 0.1);
            caixa(c, v3(0.0, h / 2.0, z), 1.45, h, 0.1, Mat::std(cinza("#7d7f84")).r(0.8));
            caixa(c, v3(0.0, h + 0.006, z + 0.02), 1.45, 0.012, 0.05, Mat::std(cinza("#c9cbcf")).r(0.6));
        }
        caixa(c, v3(0.0, 0.42, -0.93), 1.45, 0.02, 0.02, Mat::std("#3a3a3c").r(0.4).m(0.6));
        for x in [-0.7f32, 0.0, 0.7] {
            caixa(c, v3(x, 0.36, -0.93), 0.02, 0.12, 0.02, Mat::std("#3a3a3c").r(0.4).m(0.6));
        }
        placa_em_pe(c, m, nome);
        return;
    }
    if n.contains("festa") || n.contains("evento") {
        // palanque: tablado escuro com o nome na frente, trelica atras com lampadas verdes
        caixa(c, v3(0.0, 0.14, 0.05), 1.3, 0.28, 0.85, Mat::std(cinza("#1d1c1f")).r(0.6));
        caixa(c, v3(0.0, 0.285, 0.05), 1.32, 0.012, 0.87, Mat::std(cinza("#2a2a2e")).r(0.5));
        caixa(c, v3(0.45, 0.06, 0.55), 0.3, 0.12, 0.16, Mat::std(cinza("#2a2a2e")).r(0.7));
        for x in [-0.6f32, 0.6] {
            caixa(c, v3(x, 0.62, -0.32), 0.05, 0.68, 0.05, Mat::std("#4a4a4e").r(0.4).m(0.6));
        }
        caixa(c, v3(0.0, 0.96, -0.32), 1.25, 0.06, 0.06, Mat::std("#4a4a4e").r(0.4).m(0.6));
        let k = c.mat(Mat::std("#2fd16b").e("#2fd16b", 1.2));
        for x in [-0.4f32, 0.0, 0.4] {
            c.esfera(&m.e(v3(x, 0.89, -0.28), V3::default(), 1.0), 0.06, 16, k);
        }
        plano_tex(c, &m.e(v3(0.0, 0.15, 0.48), V3::default(), 1.0), 0.7, 0.13, placa_nome(&nome_placa(nome)));
        return;
    }
    if n.contains("obras") {
        // rua asfaltada com faixa tracejada, dois postes acesos e dois caminhoes
        caixa(c, v3(0.0, 0.012, 0.05), 1.9, 0.024, 0.6, Mat::std("#1f1f22").r(0.85));
        for i in 0..5 {
            caixa(c, v3(-0.76 + i as f32 * 0.38, 0.026, 0.05), 0.18, 0.004, 0.03, Mat::std("#eeeeee").r(0.6));
        }
        for x in [-0.55f32, 0.55] {
            caixa(c, v3(x, 0.45, -0.32), 0.035, 0.9, 0.035, Mat::std("#2a2a2c").r(0.5).m(0.5));
            caixa(c, v3(x, 0.89, -0.22), 0.04, 0.03, 0.22, Mat::std("#2a2a2c").r(0.5).m(0.5));
            caixa(c, v3(x, 0.86, -0.13), 0.1, 0.04, 0.08, Mat::std("#ffd27a").e("#ffcf6b", 1.5));
            caixa(c, v3(x, 0.027, -0.1), 0.42, 0.002, 0.36, Mat::std("#ffd27a").e("#ffd27a", 0.6).o(0.3));
        }
        for (x, cor) in [(-0.2f32, "#2f6b3a"), (0.45, "#e9e6df")] {
            caixa(c, v3(x, 0.17, 0.05), 0.42, 0.26, 0.3, Mat::std(cinza(cor)).r(0.5));
            caixa(c, v3(x + 0.28, 0.13, 0.05), 0.13, 0.18, 0.28, Mat::std(cinza("#d8d4cc")).r(0.45));
            caixa(c, v3(x + 0.346, 0.165, 0.05), 0.004, 0.07, 0.24, Mat::std("#16171a").r(0.2));
            let k = c.mat(Mat::std("#0e0e10").r(0.8));
            for wx in [x - 0.12, x + 0.26] {
                for wz in [-0.11f32, 0.21] {
                    c.cilindro(&m.e(v3(wx, 0.05, wz), v3(PI / 2.0, 0.0, 0.0), 1.0), 0.05, 0.05, 0.04, 14, k);
                }
            }
        }
        placa_em_pe(c, m, nome);
        return;
    }
    if tipo == "rua" || n.contains("transporte") || n.contains("mobilidade") {
        // dois onibus claros, de comprido para a camera (como no video), com janelas, para-brisa, rodas e farois
        for (x, z, rot) in [(-0.38f32, 0.12f32, PI / 2.0), (0.4, -0.2, PI / 2.0 + 0.12)] {
            let b = m.e(v3(x, 0.0, z), v3(0.0, rot, 0.0), 1.0);
            let mut peca = |pos: V3, w: f32, h: f32, d: f32, mat: Mat| {
                let k = c.mat(mat);
                c.caixa(&b.e(pos, V3::default(), 1.0), w, h, d, k);
            };
            peca(v3(0.0, 0.2, 0.0), 1.0, 0.27, 0.28, Mat::std(cinza("#a89f8e")).r(0.55));
            peca(v3(0.0, 0.345, 0.0), 0.95, 0.02, 0.25, Mat::std(cinza("#bbb2a2")).r(0.55));
            peca(v3(0.02, 0.25, 0.0), 0.8, 0.09, 0.305, Mat::std("#16171a").r(0.2).m(0.2));
            for sx in [-1.0f32, 1.0] {
                let farol = if sx > 0.0 { "#fff3c4" } else { "#c0392b" };
                peca(v3(sx * 0.476, 0.24, 0.0), 0.01, 0.13, 0.26, Mat::std("#101113").r(0.15));
                for fz in [0.1f32, -0.1] {
                    peca(v3(sx * 0.477, 0.12, fz), 0.01, 0.035, 0.05, Mat::std(farol).e(farol, 0.5));
                }
            }
            let k = c.mat(Mat::std("#0e0e10").r(0.8));
            for (wx, wz) in [(-0.3f32, 0.14f32), (0.3, 0.14), (-0.3, -0.14), (0.3, -0.14)] {
                c.cilindro(&b.e(v3(wx, 0.065, wz), v3(PI / 2.0, 0.0, 0.0), 1.0), 0.065, 0.065, 0.045, 16, k);
            }
        }
        placa_em_pe(c, m, nome);
        return;
    }
    if tipo == "predio" || n.contains("câmara") || n.contains("camara") || n.contains("administra") {
        // predio publico: degraus e alicerce claros, miolo escuro, colunata branca e laje grossa com beiral
        caixa(c, v3(0.0, 0.04, 0.03), 1.32, 0.08, 0.9, Mat::std(cinza("#a39d92")).r(0.8));
        caixa(c, v3(0.0, 0.1, 0.03), 1.22, 0.04, 0.82, Mat::std(cinza("#b9b3a7")).r(0.8));
        caixa(c, v3(0.0, 0.03, 0.5), 0.5, 0.06, 0.08, Mat::std(cinza("#b9b3a7")).r(0.8));
        caixa(c, v3(0.0, 0.36, -0.08), 1.0, 0.48, 0.48, Mat::std(cinza("#2b2b2e")).r(0.7));
        for i in 0..4 {
            caixa(c, v3(-0.36 + i as f32 * 0.24, 0.38, 0.165), 0.1, 0.16, 0.01, Mat::std("#6f8296").r(0.25).m(0.1));
        }
        caixa(c, v3(0.0, 0.25, 0.165), 0.14, 0.26, 0.01, Mat::std("#111214"));
        let k = c.mat(Mat::std(cinza("#f1ece0")).r(0.55));
        for i in 0..7 {
            let x = -0.54 + i as f32 * 0.18;
            c.cilindro(&m.e(v3(x, 0.355, 0.33), V3::default(), 1.0), 0.032, 0.036, 0.47, 14, k);
            caixa(c, v3(x, 0.135, 0.33), 0.09, 0.03, 0.09, Mat::std(cinza("#e6e0d2")).r(0.6));
        }
        caixa(c, v3(0.0, 0.66, 0.03), 1.34, 0.15, 0.9, Mat::std(cinza("#ece6d8")).r(0.6));
        caixa(c, v3(0.0, 0.745, 0.03), 1.38, 0.025, 0.94, Mat::std(cinza("#f6f2e8")).r(0.55));
        plano_tex(c, &m.e(v3(0.0, 0.66, 0.485), V3::default(), 1.0), 0.8, 0.13, placa_nome(&nome_placa(nome)));
        return;
    }
    // padrao (escola, saude, casa...): casa de paredes claras, telhado escuro de duas aguas com beiral,
    // porta, janelas com moldura e a placa verde do nome na fachada
    let moldura = cinza("#efe6d2");
    caixa(c, v3(0.0, 0.025, 0.0), 1.1, 0.05, 0.7, Mat::std(cinza("#8c8478")).r(0.85));
    caixa(c, v3(0.0, 0.26, 0.0), 1.0, 0.42, 0.6, Mat::std(cinza("#cdb994")).r(0.8));
    caixa(c, v3(0.0, 0.475, 0.0), 1.04, 0.03, 0.64, Mat::std(cinza("#e4d7bb")).r(0.7));
    let k = c.mat(Mat::std(cinza("#4a413b")).r(0.7));
    c.prisma(&m.e(v3(0.0, 0.49, 0.0), V3::default(), 1.0), 1.14, 0.27, 0.76, k);
    caixa(c, v3(0.0, 0.18, 0.304), 0.17, 0.26, 0.012, Mat::std("#17120f").r(0.6));
    caixa(c, v3(0.0, 0.18, 0.302), 0.21, 0.29, 0.008, Mat::std(moldura).r(0.6));
    let janela = |c: &mut Cena3, pos: V3, w: f32, h: f32, d: f32| {
        caixa(c, pos, w + 0.03, h + 0.03, d * 0.6, Mat::std(moldura).r(0.6));
        caixa(c, pos, w, h, d, Mat::std("#7d93aa").r(0.2).m(0.15));
        caixa(c, pos + v3(0.0, -h / 2.0 - 0.02, 0.0), w + 0.05, 0.02, d * 1.6, Mat::std(moldura).r(0.6));
    };
    for x in [-0.37f32, -0.2, 0.2, 0.37] {
        janela(c, v3(x, 0.22, 0.303), 0.11, 0.1, 0.012);
    }
    for sx in [-1.0f32, 1.0] {
        for z in [-0.13f32, 0.13] {
            janela(c, v3(sx * 0.503, 0.24, z), 0.012, 0.1, 0.11);
        }
    }
    if !nome.is_empty() {
        plano_tex(c, &m.e(v3(0.0, 0.385, 0.312), V3::default(), 1.0), 0.5, 0.1, placa_nome(&nome_placa(nome)));
    }
}

/// plaquinha do valor: moldura na cor do ano, fundo da mesma cor bem escurecido, letras claras (como no video)
pub fn placa(l1: &str, l2: &str, cor: &str) -> Arc<Tex> {
    com_cache(format!("magno-placa|{l1}|{l2}|{cor}"), || {
        let mut pm = tiny_skia::Pixmap::new(512, 190).unwrap();
        let c = Cor::hex(cor);
        ret(&mut pm, 4.0, 4.0, 504.0, 182.0, 24.0, Cor(c.0, c.1, c.2, 1.0));
        ret(&mut pm, 13.0, 13.0, 486.0, 164.0, 17.0, Cor(c.0 * 0.42, c.1 * 0.36, c.2 * 0.24, 0.94));
        let f = &fontes().arial_black;
        let mut px = 82.0;
        while f.largura(px, l1, 0.0) > 450.0 && px > 28.0 {
            px -= 4.0;
        }
        let w = f.largura(px, l1, 0.0);
        f.desenhar(&mut pm, px, 256.0 - w / 2.0, 106.0, l1, 0.0, Cor::hex("#f3e2ad"));
        let px2 = 36.0;
        let w2 = f.largura(px2, l2, 2.0);
        f.desenhar(&mut pm, px2, 256.0 - w2 / 2.0, 158.0, l2, 2.0, Cor((c.0 * 1.15).min(1.0), (c.1 * 1.1).min(1.0), c.2, 0.95));
        pm
    })
}
