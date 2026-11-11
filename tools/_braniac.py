# Ponte opcional com o Braniac (voz e Whisper ja instalados la).
# BRANIAC_RAIZ = pasta que contem a pasta braniac; o padrao e a do Carlos.
import os
import sys

RAIZ = os.environ.get("BRANIAC_RAIZ", r"C:\Users\carlo\Projetos\Agente")
if RAIZ not in sys.path:
    sys.path.insert(0, RAIZ)
