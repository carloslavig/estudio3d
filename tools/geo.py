"""Baixa as malhas do IBGE (gratis, sem chave) para a peca "mapa": estados do Brasil + municipios da UF + o municipio alvo.
Uso:  python tools/geo.py BA "Santa Cruz Cabrália"   ->  public/geo/ba-santa-cruz-cabralia.json
Os dados ficam em cache (so baixa uma vez por UF). Fonte: IBGE (servicodados.ibge.gov.br)."""
import json
import re
import sys
import unicodedata
import urllib.parse
import urllib.request
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
GEO = RAIZ / "public" / "geo"
API = "https://servicodados.ibge.gov.br/api"


def _get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "estudio3d/1"})
    with urllib.request.urlopen(req, timeout=60) as r:
        dados = r.read()
    if dados[:2] == b"\x1f\x8b":          # o IBGE responde comprimido (gzip) mesmo sem pedir
        import gzip
        dados = gzip.decompress(dados)
    return json.loads(dados.decode("utf-8"))


def _norm(t):
    return unicodedata.normalize("NFKD", t or "").encode("ascii", "ignore").decode().lower()


def slug(t):
    return re.sub(r"[^a-z0-9]+", "-", _norm(t)).strip("-")[:40] or "x"


def _arred(coords, casas=3):
    if isinstance(coords[0], (int, float)):
        return [round(coords[0], casas), round(coords[1], casas)]
    return [_arred(c, casas) for c in coords]


def _enxuto(fc, props):
    return {"type": "FeatureCollection", "features": [{"type": "Feature", "properties": props(f["properties"]), "geometry": {"type": f["geometry"]["type"], "coordinates": _arred(f["geometry"]["coordinates"])}}
                                                      for f in fc["features"]]}


def preparar(uf, municipio):
    uf = uf.strip().upper()
    estados = _get(f"{API}/v1/localidades/estados")
    est = next((e for e in estados if e["sigla"] == uf), None)
    if not est:
        raise ValueError(f"UF desconhecida: {uf}")
    munis = _get(f"{API}/v1/localidades/estados/{est['id']}/municipios")
    alvo = next((m for m in munis if _norm(m["nome"]) == _norm(municipio)), None)
    if not alvo:
        parecidos = [m["nome"] for m in munis if _norm(municipio) in _norm(m["nome"])][:5]
        raise ValueError(f"municipio '{municipio}' nao encontrado em {uf}" + (f" (parecidos: {parecidos})" if parecidos else ""))
    saida = GEO / f"{uf.lower()}-{slug(alvo['nome'])}.json"
    if saida.exists():
        return saida
    sigla_de = {str(e["id"]): e["sigla"] for e in estados}
    nomes = {str(m["id"]): m["nome"] for m in munis}
    br = _get(f"{API}/v3/malhas/paises/BR?formato=application/vnd.geo+json&qualidade=minima&intrarregiao=UF")
    mu = _get(f"{API}/v3/malhas/estados/{est['id']}?formato=application/vnd.geo+json&qualidade=minima&intrarregiao=municipio")
    dados = {"uf": uf, "cod_uf": est["id"], "nome_uf": est["nome"], "alvo": {"cod": str(alvo["id"]), "nome": alvo["nome"]},
             "estados": _enxuto(br, lambda p: {"cod": p["codarea"], "sigla": sigla_de.get(p["codarea"], "")}),
             "municipios": _enxuto(mu, lambda p: {"cod": p["codarea"], "nome": nomes.get(p["codarea"], "")}),
             "fonte": "IBGE, malhas territoriais (servicodados.ibge.gov.br)"}
    GEO.mkdir(parents=True, exist_ok=True)
    saida.write_text(json.dumps(dados, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    return saida


if __name__ == "__main__":
    try:
        print("geo gravado em", preparar(sys.argv[1], sys.argv[2]))
    except (IndexError, ValueError) as e:
        print("erro:", e or "uso: python tools/geo.py UF \"Municipio\"")
        sys.exit(1)
