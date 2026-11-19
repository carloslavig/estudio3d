"""Gera a narracao: texto falado (projetos/<p>/narracao.txt) -> public/audio/<p>.wav com a voz do Braniac.
A voz e UMA SO do comeco ao fim: se o motor falha no meio, o proximo refaz o texto inteiro (nunca mistura vozes). O motor usado fica em projetos/<p>/voz.txt.
Uso:  python tools/narrar.py 001-exemplo"""
import sys
from pathlib import Path

import _braniac  # noqa: F401  (poe o Braniac no caminho)
from braniac import router, voz

router.carregar_env()
raiz = Path(__file__).resolve().parent.parent
projeto = sys.argv[1]
texto = (raiz / "projetos" / projeto / "narracao.txt").read_text(encoding="utf-8").strip()
saida = raiz / "public" / "audio" / f"{projeto}.wav"
saida.parent.mkdir(parents=True, exist_ok=True)
wav, motor = voz.falar_wav_voz_unica(texto)
saida.write_bytes(wav)
(raiz / "projetos" / projeto / "voz.txt").write_text(motor + "\n", encoding="utf-8")
print("narracao gravada em", saida, "| voz:", motor)
