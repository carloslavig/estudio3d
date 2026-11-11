"""Gera a narracao: texto falado (projetos/<p>/narracao.txt) -> public/audio/<p>.wav usando a voz do Braniac (Gemini/Edge/Kokoro).
Uso:  python tools/narrar.py 001-exemplo"""
import sys
from pathlib import Path

import _braniac  # noqa: F401  (poe o Braniac no caminho)
from braniac import voz

raiz = Path(__file__).resolve().parent.parent
projeto = sys.argv[1]
texto = (raiz / "projetos" / projeto / "narracao.txt").read_text(encoding="utf-8").strip()
saida = raiz / "public" / "audio" / f"{projeto}.wav"
saida.parent.mkdir(parents=True, exist_ok=True)
saida.write_bytes(voz.falar_wav(texto))
print("narracao gravada em", saida)
