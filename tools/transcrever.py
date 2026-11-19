"""Transcreve a narracao e grava no roteiro: audio + legenda em blocos de ate 5 palavras com tempos reais.
Rodizio (do mais rapido ao mais lento): Whisper da Groq (API, segundos) em dois modelos -> Whisper local (~1/3 da duracao do audio).
Uso:  python tools/transcrever.py 001-exemplo [arquivo.wav] [--local]   (padrao: public/audio/<projeto>.wav; --local nao manda o audio para API)"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import _braniac  # noqa: F401
from braniac import router, voz

raiz = Path(__file__).resolve().parent.parent
pos = [a for a in sys.argv[1:] if not a.startswith("--")]
projeto = pos[0]
wav = Path(pos[1]) if len(pos) > 1 else raiz / "public" / "audio" / f"{projeto}.wav"
roteiro_arq = raiz / "projetos" / projeto / "roteiro.json"


def _juntar(trios):
    """(inicio, fim, palavra) -> lista limpa; numero quebrado pelo Whisper volta a ser um so ("3" + ",7" -> "3,7")."""
    palavras = []
    for ini, fim, w in trios:
        w = w.strip()
        if not w:
            continue
        if palavras and w[0] in ",.%:;)" and not w[0].isalpha() and (w[0] in "%)" or w[:2].replace(",", "").replace(".", "").isdigit() or len(w) > 1 and w[1].isdigit()):
            palavras[-1] = (palavras[-1][0], fim, palavras[-1][2] + w)
        else:
            palavras.append((ini, fim, w))
    return palavras


def palavras_groq(modelo):
    """Palavras com tempo pela API (ja vem com pontuacao e acento; o campo de texto corrido vem com acento estragado, por isso nao e usado)."""
    import requests
    with open(ENVIO, "rb") as f:
        r = requests.post("https://api.groq.com/openai/v1/audio/transcriptions", headers={"Authorization": "Bearer " + os.environ["GROQ_API_KEY"]},
                          files={"file": ("audio.flac", f)}, timeout=120,
                          data={"model": modelo, "language": "pt", "response_format": "verbose_json", "timestamp_granularities[]": "word"})
    r.raise_for_status()
    palavras = _juntar((p["start"], p["end"], p["word"]) for p in r.json().get("words") or [])
    if not palavras or (len(palavras) > 20 and not any(w.endswith((".", "?", "!", ",")) for _, _, w in palavras)):
        raise ValueError("resposta sem palavras ou sem pontuacao (a legenda nao fecharia nas frases)")
    return palavras


def palavras_local():
    segmentos, _ = voz._carregar_whisper().transcribe(str(wav), language="pt", word_timestamps=True, beam_size=5)
    return _juntar((p.start, p.end, p.word) for s in segmentos for p in (s.words or []))


router.carregar_env()
palavras, motor = None, "whisper local"
ENVIO = os.path.join(tempfile.gettempdir(), f"solrac-{os.getpid()}.flac")
if "--local" not in sys.argv and os.environ.get("GROQ_API_KEY"):
    # copia mono 16 kHz em flac so para o envio: o wav estereo de 3 min passa do limite de 25 MB da Groq; os tempos nao mudam
    subprocess.run(["ffmpeg", "-y", "-v", "error", "-i", str(wav), "-ac", "1", "-ar", "16000", ENVIO], timeout=300, creationflags=0x08000000 if os.name == "nt" else 0)
    for m in (() if not os.path.exists(ENVIO) else ("whisper-large-v3-turbo", "whisper-large-v3")):   # cotas separadas por modelo
        try:
            palavras, motor = palavras_groq(m), "groq " + m
            break
        except Exception as e:
            print(f"groq {m} falhou ({str(e)[:80]}); passando para o proximo", file=sys.stderr)
    if os.path.exists(ENVIO):
        os.remove(ENVIO)
if palavras is None:
    palavras = palavras_local()

falas, bloco = [], []
for ini, fim, w in palavras:
    bloco.append((ini, fim, w))
    fecha = len(bloco) >= 5 or w.endswith((".", "?", "!")) or (len(bloco) >= 3 and w.endswith(","))
    if fecha:
        falas.append({"inicio": round(bloco[0][0], 2), "fim": round(bloco[-1][1] + 0.15, 2), "texto": " ".join(b[2] for b in bloco),
                      "palavras": [{"inicio": round(b[0], 2), "fim": round(b[1], 2), "texto": b[2]} for b in bloco]})   # palavra a palavra: legenda karaoke
        bloco = []
if bloco:
    falas.append({"inicio": round(bloco[0][0], 2), "fim": round(bloco[-1][1] + 0.15, 2), "texto": " ".join(b[2] for b in bloco),
                  "palavras": [{"inicio": round(b[0], 2), "fim": round(b[1], 2), "texto": b[2]} for b in bloco]})

for a, b in zip(falas, falas[1:]):   # sem sobreposicao: a legenda some quando a proxima comeca
    a['fim'] = round(min(a['fim'], b['inicio']), 2)

roteiro = json.loads(roteiro_arq.read_text(encoding="utf-8"))
roteiro["audio"] = f"audio/{wav.name}"
roteiro["legenda"] = falas
roteiro_arq.write_text(json.dumps(roteiro, ensure_ascii=False, indent=2), encoding="utf-8")
print(f"[{motor}] {len(falas)} blocos de legenda; fala termina em {falas[-1]['fim'] if falas else 0}s. Agora ajuste as cenas (inicio/fim) a esses tempos.")
