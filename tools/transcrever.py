"""Transcreve a narracao (Whisper local) e grava no roteiro: audio + legenda em blocos de ate 5 palavras com tempos reais.
Uso:  python tools/transcrever.py 001-exemplo [arquivo.wav]   (padrao: public/audio/<projeto>.wav)"""
import json
import sys
from pathlib import Path

import _braniac  # noqa: F401
from braniac import voz

raiz = Path(__file__).resolve().parent.parent
projeto = sys.argv[1]
wav = Path(sys.argv[2]) if len(sys.argv) > 2 else raiz / "public" / "audio" / f"{projeto}.wav"
roteiro_arq = raiz / "projetos" / projeto / "roteiro.json"

modelo = voz._carregar_whisper()
segmentos, _ = modelo.transcribe(str(wav), language="pt", word_timestamps=True, beam_size=5)
palavras = [(p.start, p.end, p.word.strip()) for s in segmentos for p in (s.words or []) if p.word.strip()]

falas, bloco = [], []
for ini, fim, w in palavras:
    bloco.append((ini, fim, w))
    fecha = len(bloco) >= 5 or w.endswith((".", "?", "!")) or (len(bloco) >= 3 and w.endswith(","))
    if fecha:
        falas.append({"inicio": round(bloco[0][0], 2), "fim": round(bloco[-1][1] + 0.15, 2), "texto": " ".join(b[2] for b in bloco)})
        bloco = []
if bloco:
    falas.append({"inicio": round(bloco[0][0], 2), "fim": round(bloco[-1][1] + 0.15, 2), "texto": " ".join(b[2] for b in bloco)})

for a, b in zip(falas, falas[1:]):   # sem sobreposicao: a legenda some quando a proxima comeca
    a['fim'] = round(min(a['fim'], b['inicio']), 2)

roteiro = json.loads(roteiro_arq.read_text(encoding="utf-8"))
roteiro["audio"] = f"audio/{wav.name}"
roteiro["legenda"] = falas
roteiro_arq.write_text(json.dumps(roteiro, ensure_ascii=False, indent=2), encoding="utf-8")
print(f"{len(falas)} blocos de legenda; fala termina em {falas[-1]['fim'] if falas else 0}s. Agora ajuste as cenas (inicio/fim) a esses tempos.")
