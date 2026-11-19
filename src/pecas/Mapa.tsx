import React, { useEffect, useState } from "react";
import { AbsoluteFill, Easing, continueRender, delayRender, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";

// MAPA: Brasil -> estado -> municipio com zoom animado, sobre SATELITE aberto (NASA + Sentinel-2 cloudless 2016) quando o mosaico existe
// (tools/sat.py), ou so com as divisas do IBGE (tools/geo.py). Projecao Mercator, a mesma dos mosaicos, para tudo encaixar.
// dados: { uf: "BA", municipio: "Santa Cruz Cabrália", zoom_final?: "cidade" | "estado", pinos?: [{nome, lat, lon}], marcas?: {estado, cidade} (segundos da cena) }
type Geom = { type: "Polygon" | "MultiPolygon"; coordinates: any };
type Feat = { properties: Record<string, string>; geometry: Geom };
type GeoDados = { uf: string; cod_uf: number; nome_uf: string; alvo: { cod: string; nome: string }; estados: { features: Feat[] }; municipios: { features: Feat[] } };
type Nivel = { arquivo: string; x0: number; y0: number; x1: number; y1: number };
type Sat = { niveis: { brasil: Nivel; estado: Nivel; cidade: Nivel }; credito: string };
type Caixa = { x0: number; y0: number; x1: number; y1: number };
type Pino = { nome: string; lat: number; lon: number };

const ESC = 1000; // Mercator normalizado (0 a 1) x 1000, para o SVG trabalhar com numeros confortaveis
const pj = (lon: number, lat: number): [number, number] => {
  const l = Math.max(-85.05, Math.min(85.05, lat));
  return [((lon + 180) / 360) * ESC, (0.5 - Math.log(Math.tan(Math.PI / 4 + (l * Math.PI) / 360)) / (2 * Math.PI)) * ESC];
};
const slug = (t: string) => t.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 40) || "x";

const aneis = (g: Geom): number[][][] => (g.type === "Polygon" ? [g.coordinates] : g.coordinates).flat();
const caminho = (g: Geom): string =>
  aneis(g)
    .map((anel) => anel.map(([lon, lat], i) => { const [x, y] = pj(lon, lat); return `${i ? "L" : "M"}${x.toFixed(3)} ${y.toFixed(3)}`; }).join("") + "Z")
    .join("");
const caixaDe = (fs: Feat[]): Caixa => {
  let x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9;
  for (const f of fs) for (const anel of aneis(f.geometry)) for (const [lon, lat] of anel) {
    const [x, y] = pj(lon, lat);
    x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y);
  }
  return { x0, y0, x1, y1 };
};
const enquadrar = (c: Caixa, proporcao: number, folga: number): Caixa => {
  const cx = (c.x0 + c.x1) / 2, cy = (c.y0 + c.y1) / 2;
  let w = (c.x1 - c.x0) * (1 + folga), h = (c.y1 - c.y0) * (1 + folga);
  if (w / h < proporcao) w = h * proporcao; else h = w / proporcao;
  return { x0: cx - w / 2, y0: cy - h / 2, x1: cx + w / 2, y1: cy + h / 2 };
};
const mistura = (a: Caixa, b: Caixa, t: number): Caixa => ({ x0: a.x0 + (b.x0 - a.x0) * t, y0: a.y0 + (b.y0 - a.y0) * t, x1: a.x1 + (b.x1 - a.x1) * t, y1: a.y1 + (b.y1 - a.y1) * t });

const suave = (frame: number, de: number, ate: number) => interpolate(frame, [de, ate], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: Easing.inOut(Easing.cubic) });
const carregarImagem = (src: string) => new Promise<void>((ok) => { const im = new Image(); im.onload = () => ok(); im.onerror = () => ok(); im.src = src; });

export const Mapa: React.FC<{ dados?: Record<string, unknown>; cor: string; texto: string; fonte: string }> = ({ dados, cor, texto, fonte }) => {
  const frame = useCurrentFrame();
  const { durationInFrames, fps } = useVideoConfig();
  const uf = String(dados?.uf ?? "BA").toUpperCase();
  const municipio = String(dados?.municipio ?? "");
  const so_estado = dados?.zoom_final === "estado";
  const cheia = !!dados?.tela_cheia;   // padrao Magno: satelite ocupa a tela inteira, sem carta inclinada
  const L = cheia ? 1080 : 960, A = cheia ? 1920 : 900; // area do mapa em pixels
  const pinos = (Array.isArray(dados?.pinos) ? (dados!.pinos as Pino[]) : []).filter((p) => typeof p?.lat === "number" && typeof p?.lon === "number").slice(0, 8);
  const [geo, setGeo] = useState<GeoDados | null>(null);
  const [sat, setSat] = useState<Sat | null>(null);
  const [espera] = useState(() => delayRender("carregando mapa"));
  useEffect(() => {
    const base = `geo/${uf.toLowerCase()}-${slug(municipio)}`;
    (async () => {
      try {
        const g = await (await fetch(staticFile(`${base}.json`))).json();
        let s: Sat | null = null;
        try {
          const r = await fetch(staticFile(`${base}.sat.json`));
          if (r.ok) s = await r.json();
        } catch { /* sem satelite: segue so com as divisas */ }
        if (s) await Promise.all(Object.values(s.niveis).map((n) => carregarImagem(staticFile(n.arquivo))));
        setGeo(g);
        setSat(s);
      } catch (e) {
        console.error("mapa sem dados (rode tools/geo.py e tools/sat.py):", e);
      } finally {
        continueRender(espera);
      }
    })();
  }, [uf, municipio, espera]);
  if (!geo) return null;

  const fim = durationInFrames;
  const estadoAlvo = geo.estados.features.filter((f) => f.properties.cod === String(geo.cod_uf));
  const cidade = geo.municipios.features.filter((f) => f.properties.cod === geo.alvo.cod);
  const proporcao = L / A;
  const v0 = enquadrar(caixaDe(geo.estados.features), proporcao, 0.04);
  const v1 = enquadrar(caixaDe(estadoAlvo), proporcao, 0.1);
  const v2 = enquadrar(caixaDe(cidade), proporcao, 0.7);
  const marcas = (dados?.marcas ?? {}) as { estado?: number; cidade?: number };
  const mE = typeof marcas.estado === "number" ? marcas.estado * fps : fim * 0.38;
  const mC = typeof marcas.cidade === "number" ? Math.max(marcas.cidade * fps, mE + 1.4 * fps) : fim * 0.72;
  const e1 = suave(frame, mE - 0.9 * fps, mE + 0.6 * fps);
  const e2 = so_estado ? 0 : suave(frame, mC - 0.9 * fps, mC + 0.6 * fps);
  const v = mistura(mistura(v0, v1, e1), v2, e2);
  const centro = ((): [number, number] => { const c = caixaDe(cidade); return [(c.x0 + c.x1) / 2, (c.y0 + c.y1) / 2]; })();
  const larguraVista = v.x1 - v.x0;
  const pulso = 1 + 0.25 * Math.sin(frame * 0.25);
  const marcador = interpolate(frame, [mE - 0.2 * fps, mE + 0.8 * fps], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const nivel = e2 > 0.5 ? geo.alvo.nome : e1 > 0.5 ? geo.nome_uf : "Brasil";
  const entrada = interpolate(frame, [0, 14], [0, 1], { extrapolateRight: "clamp" });
  const comSat = !!sat;
  const img = (n: Nivel, o: number) => (o > 0.01 ? <image key={n.arquivo} href={staticFile(n.arquivo)} x={n.x0 * ESC} y={n.y0 * ESC} width={(n.x1 - n.x0) * ESC} height={(n.y1 - n.y0) * ESC} preserveAspectRatio="none" opacity={o} /> : null);
  const pinoOp = interpolate(e2, [0.55, 1], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const px = larguraVista / L; // unidades do SVG por pixel da tela

  return (
    <AbsoluteFill style={{ alignItems: "center", paddingTop: cheia ? 0 : 385, opacity: entrada }}>
      <div style={{ width: L, height: A, perspective: 1800 }}>
        <div style={cheia ? { width: L, height: A, overflow: "hidden" } : { width: L, height: A, transform: "rotateX(22deg)", transformOrigin: "50% 80%", filter: "drop-shadow(0 38px 30px rgba(0,0,0,0.55))", borderRadius: 18, overflow: "hidden" }}>
          <svg width={L} height={A} viewBox={`${v.x0} ${v.y0} ${v.x1 - v.x0} ${v.y1 - v.y0}`} style={{ background: "#0a1a33" }}>
            {comSat ? <>{img(sat!.niveis.brasil, 1)}{img(sat!.niveis.estado, e1)}{img(sat!.niveis.cidade, e2)}</> : null}
            {geo.estados.features.map((f) => {
              const alvo = f.properties.cod === String(geo.cod_uf);
              return (
                <path key={f.properties.cod} d={caminho(f.geometry)}
                  fill={alvo ? (comSat ? "none" : cor) : comSat ? "#06142a" : "#2a4a9a"} fillOpacity={alvo ? 0.35 : comSat ? 0.5 * e1 : 0.55 * (1 - 0.85 * e1)}
                  stroke={alvo ? cor : "#ffffff"} strokeOpacity={alvo ? 1 : 0.65} strokeWidth={alvo ? 3.2 : 1.3} vectorEffect="non-scaling-stroke" style={alvo ? { filter: `drop-shadow(0 0 8px ${cor})` } : undefined} />
              );
            })}
            {geo.municipios.features.map((f) => {
              if (f.properties.cod === geo.alvo.cod) return null;
              return <path key={f.properties.cod} d={caminho(f.geometry)} fill="none" stroke="#ffffff" strokeOpacity={0.4 * e1 * (1 - 0.4 * e2)} strokeWidth={0.9} vectorEffect="non-scaling-stroke" />;
            })}
            {cidade.map((f) => (
              <path key={f.properties.cod} d={caminho(f.geometry)} fill={cor} fillOpacity={comSat ? 0.1 + 0.15 * e1 : 0.35 + 0.65 * e1} stroke="#ffffff" strokeWidth={e2 > 0 ? 4 : 2.4} vectorEffect="non-scaling-stroke" style={{ filter: `drop-shadow(0 0 ${6 + 10 * e2}px ${cor})` }} />
            ))}
            <circle cx={centro[0]} cy={centro[1]} r={larguraVista * 0.03 * pulso} fill="none" stroke={cor} strokeWidth={4} strokeOpacity={marcador * 0.8} vectorEffect="non-scaling-stroke" />
            <circle cx={centro[0]} cy={centro[1]} r={larguraVista * 0.011} fill="#ffffff" fillOpacity={marcador} stroke={cor} strokeWidth={5} vectorEffect="non-scaling-stroke" />
            {cheia && e2 > 0.3 ? <text x={centro[0]} y={centro[1] - larguraVista * 0.045} textAnchor="middle" fontSize={46 * px} fontWeight={900} fill="#ffffff" stroke="#000000" strokeWidth={6 * px} paintOrder="stroke" fontFamily="'Courier New', monospace" opacity={Math.min(1, (e2 - 0.3) * 3)} letterSpacing={2 * px}>{geo.alvo.nome.toUpperCase()}</text> : null}
            {pinos.map((p, i) => {
              const [x, y] = pj(p.lon, p.lat);
              const pop = interpolate(frame - i * 6, [0, 10], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }) * pinoOp;
              return (
                <g key={i} opacity={pop}>
                  <circle cx={x} cy={y} r={11 * px} fill="#ef4444" stroke="#ffffff" strokeWidth={3 * px} />
                  <text x={(x - v.x0) / larguraVista > 0.55 ? x - 18 * px : x + 18 * px} y={y + 8 * px} textAnchor={(x - v.x0) / larguraVista > 0.55 ? "end" : "start"} fontSize={30 * px} fill="#ffffff" stroke="#000000" strokeWidth={5 * px} paintOrder="stroke" fontFamily={fonte}>{String(p.nome ?? "").slice(0, 30)}</text>
                </g>
              );
            })}
          </svg>
        </div>
      </div>
      {cheia ? <Enfeites dados={dados} t={frame / fps} /> : null}
      <div style={{ display: cheia ? "none" : "block", position: "absolute", top: 1300, width: "100%", textAlign: "center", fontFamily: fonte, color: texto }}>
        <div style={{ fontSize: 30, letterSpacing: 6, opacity: 0.7, textTransform: "uppercase" }}>{e2 > 0.5 ? `${geo.nome_uf} · município` : e1 > 0.5 ? "Estado" : "País"}</div>
        <div style={{ fontSize: 68, lineHeight: 1.1, textShadow: `0 4px 24px ${cor}88` }}>{nivel}</div>
      </div>
      {comSat ? <div style={{ position: "absolute", bottom: cheia ? 24 : 70, width: "100%", textAlign: "center", fontFamily: "system-ui, sans-serif", fontSize: cheia ? 15 : 21, color: texto, opacity: cheia ? 0.35 : 0.55, padding: "0 70px", boxSizing: "border-box" }}>{sat!.credito}</div> : null}
    </AbsoluteFill>
  );
};

// Enfeites do padrao Magno sobre o mapa: pilula "ARRECADOU R$ X · ANO" (a mais recente), etiqueta do TCM e faixa verde da prefeitura. Tempos em segundos da cena.
const MONO = "'Courier New', 'Consolas', monospace";
type Marca = { rotulo?: string; ano?: string; t?: number };
const Enfeites: React.FC<{ dados?: Record<string, unknown>; t: number }> = ({ dados, t }) => {
  const plaq = (Array.isArray(dados?.plaquinhas) ? (dados!.plaquinhas as Marca[]) : []).filter((p) => (p.t ?? 0) <= t).pop();
  const etq = dados?.etiqueta as Marca | undefined, fx = dados?.faixa as Marca | undefined;
  const ve = (m?: Marca) => !!m?.rotulo && t >= (m.t ?? 0);
  const pilula = { fontFamily: MONO, fontWeight: 900 as const, letterSpacing: 2, color: "#fff", textTransform: "uppercase" as const, borderRadius: 12, padding: "10px 24px" };
  return (
    <AbsoluteFill style={{ alignItems: "center", pointerEvents: "none" }}>
      {ve(etq) ? <div style={{ ...pilula, position: "absolute", top: 660, fontSize: 30, background: "rgba(12,12,14,0.85)", border: "3px solid #fff" }}>{etq!.rotulo}</div>
        : plaq?.rotulo ? <div style={{ ...pilula, position: "absolute", top: 640, fontSize: 32, background: "rgba(12,12,14,0.85)", border: "3px solid #fff" }}>ARRECADOU {plaq.rotulo}{plaq.ano ? ` · ${plaq.ano}` : ""}</div> : null}
      {ve(fx) ? <div style={{ ...pilula, position: "absolute", top: 1290, fontSize: 30, background: "rgba(46,160,90,0.95)" }}>{fx!.rotulo}</div> : null}
    </AbsoluteFill>
  );
};
