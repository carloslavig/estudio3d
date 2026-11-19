"""Monta os mosaicos de SATELITE da peca "mapa" (Brasil, estado, municipio) a partir de fontes abertas, depois de tools/geo.py.
Uso:  python tools/sat.py BA "Santa Cruz Cabrália"   ->  public/geo/sat/<uf>-<slug>-{brasil,estado,cidade}.jpg + public/geo/<uf>-<slug>.sat.json
Fontes (licencas abertas, os creditos vao no rodape do video):
  - Brasil inteiro: NASA Blue Marble com relevo e batimetria (GIBS, dominio publico).
  - Estado e municipio: Sentinel-2 cloudless 2016 by EOX IT Services GmbH (contem dados Copernicus Sentinel modificados 2016), CC BY 4.0.
    Atencao: os mosaicos de 2017 em diante da EOX sao so para uso NAO comercial; por isso aqui se usa SO o de 2016 (camada s2cloudless_3857).
Cada mosaico fica em coordenadas Mercator normalizadas (0 a 1) no .sat.json, as mesmas que a peca usa para desenhar as divisas por cima."""
import json
import math
import sys
import time
import urllib.request
from io import BytesIO
from pathlib import Path

from PIL import Image

RAIZ = Path(__file__).resolve().parent.parent
GEO = RAIZ / "public" / "geo"
SAT = GEO / "sat"
NASA = "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_ShadedRelief_Bathymetry/default/GoogleMapsCompatible_Level8/{z}/{y}/{x}.jpeg"
EOX = "https://tiles.maps.eox.at/wmts/1.0.0/s2cloudless_3857/default/g/{z}/{y}/{x}.jpg"
CREDITO = "Imagens: NASA · Sentinel-2 cloudless 2016 by EOX IT Services GmbH (contém dados Copernicus Sentinel modificados 2016) · Divisas: IBGE"


def merc(lon, lat):
    lat = max(-85.05, min(85.05, lat))
    return (lon + 180) / 360, 0.5 - math.log(math.tan(math.pi / 4 + math.radians(lat) / 2)) / (2 * math.pi)


def caixa(feats):
    xs, ys = [], []
    for f in feats:
        g = f["geometry"]
        for poli in ([g["coordinates"]] if g["type"] == "Polygon" else g["coordinates"]):
            for lon, lat in poli[0]:
                x, y = merc(lon, lat)
                xs.append(x)
                ys.append(y)
    return min(xs), min(ys), max(xs), max(ys)


def tile(url, z, x, y, tentativas=3):
    for i in range(tentativas):
        try:
            req = urllib.request.Request(url.format(z=z, x=x, y=y), headers={"User-Agent": "estudio3d/1 (video mapas)"})
            with urllib.request.urlopen(req, timeout=30) as r:
                return Image.open(BytesIO(r.read())).convert("RGB")
        except Exception:
            time.sleep(1 + i)
    return Image.new("RGB", (256, 256), (10, 20, 40))   # tile ausente (mar/sem dado): fundo escuro


def mosaico(url, z, bbox, nome, folga):
    """Baixa os tiles que cobrem bbox (Mercator normalizado, com folga) e devolve o arquivo e a caixa realmente coberta."""
    x0, y0, x1, y1 = bbox
    w, h = x1 - x0, y1 - y0
    x0, x1, y0, y1 = x0 - w * folga, x1 + w * folga, y0 - h * folga, y1 + h * folga
    n = 2 ** z
    tx0, tx1, ty0, ty1 = int(x0 * n), int(x1 * n), int(y0 * n), int(y1 * n)
    img = Image.new("RGB", ((tx1 - tx0 + 1) * 256, (ty1 - ty0 + 1) * 256))
    for ty in range(ty0, ty1 + 1):
        for tx in range(tx0, tx1 + 1):
            img.paste(tile(url, z, tx, ty), ((tx - tx0) * 256, (ty - ty0) * 256))
    SAT.mkdir(parents=True, exist_ok=True)
    img.save(SAT / nome, quality=88)
    return {"arquivo": f"geo/sat/{nome}", "x0": tx0 / n, "y0": ty0 / n, "x1": (tx1 + 1) / n, "y1": (ty1 + 1) / n}


def escolher_z(url_max, bbox, folga, largura_max_tiles=6):
    w = (bbox[2] - bbox[0]) * (1 + 2 * folga)
    for z in range(url_max, 3, -1):
        if w * 2 ** z <= largura_max_tiles:
            return z
    return 4


def preparar(uf, municipio):
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import geo as G
    arq = G.preparar(uf, municipio)
    d = json.loads(arq.read_text(encoding="utf-8"))
    saida = arq.with_suffix(".sat.json")
    if saida.exists():
        return saida
    base = arq.stem
    estado = [f for f in d["estados"]["features"] if f["properties"]["cod"] == str(d["cod_uf"])]
    cidade = [f for f in d["municipios"]["features"] if f["properties"]["cod"] == d["alvo"]["cod"]]
    brasil = caixa(d["estados"]["features"])
    cx, ez = caixa(cidade), caixa(estado)
    niveis = {
        "brasil": mosaico(NASA, 5 if (brasil[2] - brasil[0]) * 32 <= 6 else 4, brasil, f"{base}-brasil.jpg", 0.06),
        "estado": mosaico(EOX, escolher_z(8, ez, 0.12, 7), ez, f"{base}-estado.jpg", 0.12),
        "cidade": mosaico(EOX, escolher_z(12, cx, 0.9, 6), cx, f"{base}-cidade.jpg", 0.9),
    }
    saida.write_text(json.dumps({"niveis": niveis, "credito": CREDITO}, ensure_ascii=False), encoding="utf-8")
    return saida


def preparar_perto(uf, municipio):
    """Visual "magno" do motor-rs: o voo chega bem perto da cidade (ela ocupa ~28% da largura do quadro 9:16), mais perto do que o
    nivel "cidade" cobre. Baixa 3 mosaicos centrados no quadro final (z 8, 9 e 10) em <base>.sat-perto.json; o .sat.json fica igual."""
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import geo as G
    arq = G.preparar(uf, municipio)
    d = json.loads(arq.read_text(encoding="utf-8"))
    saida = arq.with_name(arq.stem + ".sat-perto.json")
    if saida.exists():
        return saida
    cidade = [f for f in d["municipios"]["features"] if f["properties"]["cod"] == d["alvo"]["cod"]]
    x0, y0, x1, y1 = caixa(cidade)
    w1 = (x1 - x0) / 0.27          # mesma conta do motor-rs (mapa.rs, fn voo)
    h1 = w1 / (1080 / 1920)
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2 - 0.08 * h1
    niveis = {}
    for z, k in ((8, 5.5), (9, 2.8), (10, 1.3)):     # cada nivel cobre as vistas em que a resolucao dele ainda basta
        caixa_z = (cx - w1 * k / 2, cy - h1 * k / 2, cx + w1 * k / 2, cy + h1 * k / 2)
        niveis[f"z{z}"] = mosaico(EOX, z, caixa_z, f"{arq.stem}-perto-z{z}.jpg", 0.0)
    saida.write_text(json.dumps({"niveis": niveis, "credito": CREDITO}, ensure_ascii=False), encoding="utf-8")
    return saida


if __name__ == "__main__":
    try:
        if "--perto" in sys.argv:
            a = [x for x in sys.argv[1:] if x != "--perto"]
            print("satelite de perto gravado em", preparar_perto(a[0], a[1]))
            sys.exit(0)
        print("satelite gravado em", preparar(sys.argv[1], sys.argv[2]))
    except (IndexError, ValueError) as e:
        print("erro:", e or 'uso: python tools/sat.py UF "Municipio"')
        sys.exit(1)
