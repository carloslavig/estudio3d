// Mesmo roteiro.json do estudio (src/tipos.ts) e mesmas cores de src/estilos.ts.
use crate::desenho::Cor;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Palavra {
    pub inicio: f64,
    pub fim: f64,
    pub texto: String,
}

#[derive(Deserialize)]
pub struct Fala {
    pub inicio: f64,
    pub fim: f64,
    pub texto: String,
    #[serde(default)]
    pub palavras: Vec<Palavra>,
}

#[derive(Deserialize)]
pub struct Cena {
    pub inicio: f64,
    pub fim: f64,
    #[serde(default)]
    pub titulo: Option<String>,
    pub peca: String,
    #[serde(default)]
    pub dados: serde_json::Value,
}

#[derive(Deserialize)]
pub struct Cabecalho {
    pub titulo: String,
    pub arroba: Option<String>,
    pub fonte: Option<String>,
    pub escala: Option<String>,
}

#[derive(Deserialize)]
pub struct Roteiro {
    pub cabecalho: Option<Cabecalho>,
    #[allow(dead_code)]
    pub titulo: String,
    pub estilo: String,
    pub audio: Option<String>,
    pub legenda: Vec<Fala>,
    pub cenas: Vec<Cena>,
    /// "magno": visual recriado a partir do video modelo do Magno (so no motor-rs); sem ele, igual ao Remotion
    #[serde(default)]
    pub visual: Option<String>,
}

pub const FPS: f64 = 30.0;
pub const LARG: u32 = 1080;
pub const ALT: u32 = 1920;

/// Pecas que este motor ja desenha; as outras ainda so existem no motor Remotion.
pub const PECAS_PRONTAS: &[&str] = &["numero", "linha", "mapa", "maquete", "barras"];

impl Roteiro {
    pub fn magno(&self) -> bool {
        self.visual.as_deref() == Some("magno")
    }

    // igual a duracaoEmQuadros do Estudio.tsx: fim mais tardio + 0,5 s de respiro
    pub fn quadros(&self) -> u32 {
        let fim = self
            .cenas
            .iter()
            .map(|c| c.fim)
            .chain(self.legenda.iter().map(|f| f.fim))
            .fold(1.0, f64::max);
        // magno: o video acaba junto com a ultima cena (sem o meio segundo de tela preta no fim)
        (fim * FPS).ceil() as u32 + if self.magno() { 0 } else { 15 }
    }
}

pub struct Estilo {
    pub fundo_a: Cor,
    pub fundo_b: Cor,
    pub destaque: Cor,
    pub texto: Cor,
    pub karaoke: bool,
    pub karaoke_ativa: Option<Cor>,
    /// arquivos em C:\Windows\Fonts: (normal, negrito)
    pub fontes: (&'static str, &'static str),
}

pub fn estilo(chave: &str) -> Estilo {
    match chave {
        "quente" => Estilo {
            fundo_a: Cor::hex("#1f0a0a"),
            fundo_b: Cor::hex("#5a1b1b"),
            destaque: Cor::hex("#fbbf24"),
            texto: Cor::hex("#fff7ed"),
            karaoke: false,
            karaoke_ativa: None,
            fontes: ("ariblk.ttf", "ariblk.ttf"),
        },
        // shortcut: Inter nao esta instalada, entao usa Segoe UI como o Remotion; instalar Inter quando der
        "mesa" => Estilo {
            fundo_a: Cor::hex("#2a221b"),
            fundo_b: Cor::hex("#0d0b0a"),
            destaque: Cor::hex("#f4c542"),
            texto: Cor::hex("#F0EBE1"),
            karaoke: true,
            karaoke_ativa: Some(Cor::hex("#F0EBE1")),
            fontes: ("segoeui.ttf", "segoeuib.ttf"),
        },
        _ => Estilo {
            fundo_a: Cor::hex("#0b1020"),
            fundo_b: Cor::hex("#1b2a5a"),
            destaque: Cor::hex("#38bdf8"),
            texto: Cor::hex("#f8fafc"),
            karaoke: false,
            karaoke_ativa: None,
            fontes: ("ariblk.ttf", "ariblk.ttf"),
        },
    }
}

/// Legenda.tsx/juntarNumeros: "3" + ",7" vira "3,7" e "R$" gruda no numero.
pub fn juntar_numeros(palavras: &[Palavra]) -> Vec<Palavra> {
    let mut saida: Vec<Palavra> = Vec::new();
    for p in palavras {
        if let Some(ult) = saida.last_mut() {
            let frag = (p.texto.starts_with([',', '.', '%'])
                && ult.texto.ends_with(|c: char| c.is_ascii_digit() || c == '$'))
                || ult.texto == "R$";
            if frag {
                ult.texto.push_str(&p.texto);
                ult.fim = p.fim;
                continue;
            }
        }
        saida.push(p.clone());
    }
    saida
}

#[cfg(test)]
mod testes {
    use super::*;
    fn p(t: &str) -> Palavra {
        Palavra { inicio: 0.0, fim: 1.0, texto: t.into() }
    }
    #[test]
    fn junta_numeros_como_o_remotion() {
        let j = juntar_numeros(&[p("R$"), p("3"), p(",7"), p("bi"), p("Educação,"), p("2")]);
        let t: Vec<_> = j.iter().map(|x| x.texto.as_str()).collect();
        assert_eq!(t, ["R$3,7", "bi", "Educação,", "2"]);
    }
}
