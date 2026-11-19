// motor-rs: render do Solrac Estudio sem navegador. Roda em PARALELO ao Remotion (nao substitui nada).
// Uso (na pasta Estudio3D):
//   motor-rs <projeto | caminho/roteiro.json>             -> edicoes/<projeto>/video-rs.mp4
//   motor-rs <projeto> --quadro=4.5                        -> edicoes/<projeto>/quadro-rs-4.5.png
//   opcoes: --estrito (recusa com codigo 3 se faltar peca)  --cpu (libx264 em vez da placa NVIDIA)  --saida=arquivo.mp4  --ate=10 (so os primeiros N segundos)
mod desenho;
mod gpu;
mod mapa;
mod maquete;
mod maquete_magno;
mod quadro;
mod roteiro;
mod tres;

use rayon::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

fn opcao<'a>(args: &'a [String], nome: &str) -> Option<&'a str> {
    args.iter().find_map(|a| a.strip_prefix(&format!("--{nome}=")))
}

fn raiz_do_estudio() -> PathBuf {
    if let Ok(r) = std::env::var("ESTUDIO_RAIZ") {
        return PathBuf::from(r);
    }
    let aqui = std::env::current_dir().unwrap_or_default();
    // funciona tanto em Estudio3D quanto em Estudio3D/motor-rs
    if aqui.join("projetos").is_dir() { aqui } else { aqui.parent().map(Path::to_path_buf).unwrap_or(aqui) }
}

fn main() {
    if let Err(e) = rodar() {
        eprintln!("motor-rs: {e}");
        std::process::exit(1);
    }
}

fn rodar() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let alvo = args.iter().find(|a| !a.starts_with("--")).ok_or("diga o projeto: motor-rs <pasta em projetos/> [--quadro=S] [--cpu]")?;
    let raiz = raiz_do_estudio();
    let _ = mapa::RAIZ.set(raiz.clone());
    let caminho = if Path::new(alvo).is_file() { PathBuf::from(alvo) } else { raiz.join("projetos").join(alvo).join("roteiro.json") };
    let nome = caminho.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "video".into());
    let texto = std::fs::read_to_string(&caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
    let r: roteiro::Roteiro = serde_json::from_str(&texto).map_err(|e| format!("roteiro invalido: {e}"))?;

    // barras so existe aqui no formato mesa (Comparativo); as outras ainda so no Remotion
    let faltam: Vec<&str> = r
        .cenas
        .iter()
        .filter(|c| !roteiro::PECAS_PRONTAS.contains(&c.peca.as_str()) || (c.peca == "barras" && !c.dados["mesa"].as_bool().unwrap_or(false)))
        .map(|c| c.peca.as_str())
        .collect();
    if !faltam.is_empty() {
        let mut u = faltam.clone();
        u.sort();
        u.dedup();
        if args.iter().any(|a| a == "--estrito") {
            // o Braniac usa --estrito: sem todas as pecas, sai com codigo 3 e ele renderiza no Remotion
            eprintln!("motor-rs ainda nao faz: {}", u.join(", "));
            std::process::exit(3);
        }
        eprintln!("aviso: pecas ainda nao portadas (aparece so o fundo): {}", u.join(", "));
    }

    let audio = r.audio.as_ref().map(|a| raiz.join("public").join(a)).filter(|a| a.is_file());
    let motor = quadro::Motor::novo(r, &raiz)?;
    let pasta = raiz.join("edicoes").join(&nome);
    std::fs::create_dir_all(&pasta).map_err(|e| e.to_string())?;

    if let Some(s) = opcao(&args, "quadro") {
        let seg: f64 = s.parse().map_err(|_| "--quadro precisa de segundos, ex.: --quadro=4.5")?;
        let ini = Instant::now();
        let pm = motor.quadro((seg * roteiro::FPS).round() as u32);
        let primeiro = ini.elapsed().as_secs_f64() * 1000.0;
        let ini = Instant::now();
        let _ = motor.quadro((seg * roteiro::FPS).round() as u32 + 1);
        eprintln!("quadro: {primeiro:.0} ms (primeiro, com cargas) / {:.0} ms (seguinte)", ini.elapsed().as_secs_f64() * 1000.0);
        let saida = pasta.join(format!("quadro-rs-{s}.png"));
        pm.save_png(&saida).map_err(|e| e.to_string())?;
        println!("{}", saida.display());
        return Ok(());
    }

    let total = match opcao(&args, "ate") {
        Some(s) => ((s.parse::<f64>().map_err(|_| "--ate precisa de segundos")? * roteiro::FPS) as u32).min(motor.total),
        None => motor.total,
    };
    let saida = opcao(&args, "saida").map(PathBuf::from).unwrap_or_else(|| pasta.join("video-rs.mp4"));
    let cpu = args.iter().any(|a| a == "--cpu");

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-hide_banner", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"])
        .arg(format!("{}x{}", roteiro::LARG, roteiro::ALT))
        .args(["-r", "30", "-i", "-"]);
    if let Some(a) = &audio {
        cmd.arg("-i").arg(a).args(["-map", "0:v", "-map", "1:a", "-c:a", "aac", "-b:a", "192k"]);
    }
    // tamanho do arquivo: no oficial (1080 ou mais) a taxa sai da duracao para o mp4 ficar perto de ESTUDIO_MB (padrao 75 MB,
    // o Carlos quer 70-80 e nunca 90; o NVENC entrega ~12% abaixo do pedido, entao o padrao pede 85 para sair ~75). Teste leve (ESTUDIO_SAIDA menor que 1080) continua por qualidade (cq)
    let teste = std::env::var("ESTUDIO_SAIDA").ok().and_then(|v| v.parse::<u32>().ok()).is_some_and(|w| w < roteiro::LARG);
    let alvo_mb: f64 = std::env::var("ESTUDIO_MB").ok().and_then(|v| v.parse().ok()).unwrap_or(85.0);
    let kbps = taxa_kbps(alvo_mb, total as f64 / roteiro::FPS as f64, audio.is_some());
    let enc = if cpu { "libx264".to_string() } else { encoder() };
    match (enc.as_str(), teste) {
        ("libx264", true) => {
            cmd.args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "18"]);
        }
        ("libx264", false) => {
            cmd.args(["-c:v", "libx264", "-preset", "veryfast"]).args(["-b:v", &format!("{kbps}k"), "-maxrate", &format!("{}k", kbps * 3 / 2), "-bufsize", &format!("{}k", kbps * 2)]);
        }
        ("h264_nvenc", true) => {
            cmd.args(["-c:v", "h264_nvenc", "-preset", "p5", "-rc", "vbr", "-cq", "19", "-b:v", "0"]);
        }
        ("h264_nvenc", false) => {
            cmd.args(["-c:v", "h264_nvenc", "-preset", "p6", "-rc", "vbr", "-multipass", "fullres"])
                .args(["-b:v", &format!("{kbps}k"), "-maxrate", &format!("{}k", kbps * 3 / 2), "-bufsize", &format!("{}k", kbps * 2)]);
        }
        // outros encoders de placa (Apple, AMD, Intel): so a taxa, que todos entendem
        (outro, _) => {
            cmd.args(["-c:v", outro, "-b:v", &format!("{}k", if teste { 6000 } else { kbps })]);
        }
    }
    if audio.is_some() && total < motor.total {
        cmd.arg("-shortest");
    }
    // teste mais leve: ESTUDIO_SAIDA=720 grava em 720x1280 ("720p" do video em pe = 720 de largura); o quadro continua
    // desenhado em 1080 e e reduzido no encoder. Oficial: sem a variavel (1080) ou ESTUDIO_SAIDA=1440 (2K)
    if let Some(w) = std::env::var("ESTUDIO_SAIDA").ok().and_then(|v| v.parse::<u32>().ok()).filter(|w| *w != roteiro::LARG) {
        cmd.args(["-vf", &format!("scale={}:{}:flags=lanczos", w / 2 * 2, w * roteiro::ALT / roteiro::LARG / 2 * 2)]);
    }
    cmd.args(["-pix_fmt", "yuv420p"]).arg(&saida).stdin(Stdio::piped());
    let mut ff = cmd.spawn().map_err(|e| format!("ffmpeg nao abriu: {e}"))?;
    let mut entrada = ff.stdin.take().ok_or("sem stdin do ffmpeg")?;

    // quadros em paralelo (rayon) em lotes; um thread escreve no ffmpeg enquanto o proximo lote e desenhado
    let inicio = Instant::now();
    let lote = rayon::current_num_threads() as u32; // 1 quadro por nucleo: cada quadro tem 8 MB, lotes maiores so gastam memoria
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<Vec<u8>>>(1);
    let escritor = std::thread::spawn(move || -> std::io::Result<()> {
        for quadros in rx {
            for q in quadros {
                entrada.write_all(&q)?;
            }
        }
        Ok(())
    });
    let mut f = 0;
    while f < total {
        let fim = (f + lote).min(total);
        let quadros: Vec<Vec<u8>> = (f..fim).into_par_iter().map(|i| motor.quadro(i).take()).collect();
        if tx.send(quadros).is_err() {
            break; // escritor caiu (ffmpeg fechou): o erro aparece abaixo
        }
        f = fim;
    }
    drop(tx);
    let escrita = escritor.join().map_err(|_| "escritor travou")?;
    let st = ff.wait().map_err(|e| e.to_string())?;
    if let Err(e) = escrita {
        return Err(format!("ffmpeg parou de receber quadros ({e}); se for a placa de video, tente --cpu"));
    }
    if !st.success() {
        return Err(format!("ffmpeg falhou ({st}); se for a placa de video, tente --cpu"));
    }
    let s = inicio.elapsed().as_secs_f64();
    println!("{} | {} quadros ({:.1} s de video) em {:.1} s = {:.0} quadros/s", saida.display(), total, total as f64 / 30.0, s, total as f64 / s);
    Ok(())
}

/// taxa de video (kbps) para o mp4 ficar com ~`mb` megabytes: desconta o audio (192k) e ~2% de cabecalho do mp4
fn taxa_kbps(mb: f64, segundos: f64, com_audio: bool) -> u32 {
    let total = mb * 8.0 * 1024.0 * 0.98 / segundos.max(1.0);
    (total - if com_audio { 192.0 } else { 0.0 }).clamp(1500.0, 20000.0) as u32
}

#[cfg(test)]
mod teste_tamanho {
    #[test]
    fn video_de_168_s_com_75_mb_usa_uns_3_3_mbps() {
        let k = super::taxa_kbps(75.0, 168.3, true);
        assert!((3200..3400).contains(&k), "{k}");
        assert_eq!(super::taxa_kbps(75.0, 3.0, true), 20000); // video curtinho: teto, nada absurdo
    }
}

/// Encoder de video: o da placa que este ffmpeg conseguir usar de verdade (NVIDIA, Apple, AMD, Intel) ou, sem nenhum,
/// o do processador (libx264, que existe em todo ffmpeg). MOTOR_ENCODER=nome forca um.
fn encoder() -> String {
    if let Ok(e) = std::env::var("MOTOR_ENCODER") {
        return e;
    }
    // so listar nao basta: o ffmpeg lista h264_nvenc mesmo sem placa NVIDIA; cada um grava 3 quadros de teste
    let funciona = |e: &str| {
        Command::new("ffmpeg")
            .args(["-v", "quiet", "-f", "lavfi", "-i", "color=c=black:s=640x360:r=30:d=0.1", "-c:v", e, "-f", "null", "-"])
            .stdin(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    };
    ["h264_nvenc", "h264_videotoolbox", "h264_amf", "h264_qsv"].into_iter().find(|e| funciona(e)).unwrap_or("libx264").to_string()
}
