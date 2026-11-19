import React from "react";
import { AbsoluteFill, Audio, Sequence, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { Palco } from "./base/Palco";
import { Legenda } from "./base/Legenda";
import { PECAS } from "./pecas";
import { Mapa } from "./pecas/Mapa";
import { Numero } from "./pecas/Numero";
import { Linha } from "./pecas/Linha";
import { Moedas } from "./pecas/Moedas";
import { Foto } from "./pecas/Foto";
import { Maquete, Comparativo, setorAtivo, corAno, temposSetores, type SetorM } from "./pecas/Maquete";
import { estiloDe } from "./estilos";
import { FPS, type Cena, type Roteiro } from "./tipos";

const TituloDaCena: React.FC<{ texto: string; cor: string; fonte: string; destaque: string }> = ({ texto, cor, fonte, destaque }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const s = spring({ frame, fps, config: { damping: 15 } });
  return (
    <AbsoluteFill style={{ alignItems: "center", padding: "170px 110px 0" }}>
      <div
        style={{
          fontFamily: fonte,
          fontSize: 82,
          lineHeight: 1.08,
          textAlign: "center",
          color: cor,
          opacity: s,
          transform: `translateY(${interpolate(s, [0, 1], [40, 0])}px)`,
          borderBottom: `10px solid ${destaque}`,
          paddingBottom: 18,
        }}
      >
        {texto}
      </div>
    </AbsoluteFill>
  );
};

type Folha = { titulo?: string; linhas?: [string, string | number][]; total?: number | string };
const MONO_E = "'Courier New', 'Consolas', monospace";
const pilulaTopo = { position: "absolute" as const, top: 560, fontFamily: MONO_E, fontWeight: 800 as const, fontSize: 30, letterSpacing: 2, textTransform: "uppercase" as const, borderRadius: 12, padding: "10px 24px", maxWidth: 940, textAlign: "center" as const };
const valorBR = (v: string | number) => (typeof v === "number" ? "R$ " + v.toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : v);

// padrao Magno: folha "PDF" meio transparente no canto, com a lista item a item (nao e falada)
const FolhaPDF: React.FC<{ folha: Folha; desde: number; t: number }> = ({ folha, desde, t }) => {
  // regra do Magno: so ~1,8 a 2,5 s na tela, entra e sai suave
  const op = interpolate(t - desde, [0, 0.3, 2.0, 2.4], [0, 0.9, 0.9, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  if (op <= 0) return null;
  return (
    <div style={{ position: "absolute", left: 90, top: 1010, width: 440, background: "rgba(250,248,242,0.9)", borderRadius: 10, borderTop: "10px solid #2FA36B", padding: "14px 18px", opacity: op, fontFamily: MONO_E, color: "#1c1c1c", boxShadow: "0 10px 30px rgba(0,0,0,0.5)" }}>
      <div style={{ fontFamily: "Inter, Segoe UI, Arial, sans-serif", fontWeight: 900, fontSize: 30, lineHeight: 1.1, paddingBottom: 8, marginBottom: 8, borderBottom: "1px solid #ccc" }}>{folha.titulo}</div>
      {(folha.linhas ?? []).slice(0, 8).map(([nome, v], i) => (
        <div key={i} style={{ display: "flex", justifyContent: "space-between", gap: 10, fontSize: 13, lineHeight: 1.45, borderBottom: "1px solid #ddd" }}>
          <span style={{ overflow: "hidden", whiteSpace: "nowrap", textOverflow: "ellipsis" }}>{String(nome)}</span><span style={{ whiteSpace: "nowrap", fontWeight: 700 }}>{valorBR(v)}</span>
        </div>
      ))}
      {folha.total !== undefined ? <div style={{ textAlign: "right", fontWeight: 900, fontSize: 15, marginTop: 6 }}>TOTAL {valorBR(folha.total)}</div> : null}
    </div>
  );
};

const EtiquetaMaquete: React.FC<{ dados?: Record<string, unknown>; fonte: string }> = ({ dados }) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  const t = frame / fps;
  const setor = setorAtivo(dados, t, durationInFrames / fps) as (ReturnType<typeof setorAtivo> & { folha?: Folha }) | null;
  const destaque = (Array.isArray(dados?.destaques) ? (dados!.destaques as { rotulo?: string; t?: number; cor?: string }[]) : []).filter((d) => (d.t ?? 0) <= t).pop();
  const tempos = temposSetores((Array.isArray(dados?.setores) ? dados!.setores : []) as SetorM[], durationInFrames / fps);
  const iSetor = setor ? ((dados!.setores as SetorM[]).indexOf(setor as SetorM)) : -1;
  const tReceita = typeof (dados?.arrecadou as { t?: number } | undefined)?.t === "number" ? (dados!.arrecadou as { t: number }).t : 0.5;
  const folha = setor?.folha ? { f: setor.folha, desde: tempos[iSetor] ?? 0 } : dados?.folha_receita ? { f: dados.folha_receita as Folha, desde: tReceita } : null;
  if (dados?.visao_geral) return null;
  return (
    <AbsoluteFill style={{ alignItems: "center", pointerEvents: "none" }}>
      {setor ? <div style={{ ...pilulaTopo, color: "#fff", background: "rgba(16,16,20,0.82)" }}>{setor.nome}: {setor.rotulo ?? ""}</div>
        : destaque?.rotulo ? <div style={{ ...pilulaTopo, color: "#fff", background: destaque.cor ? corAno(destaque.cor) : "rgba(16,16,20,0.82)" }}>{destaque.rotulo}</div>
        : dados?.titulo ? <div style={{ ...pilulaTopo, color: "#d8d2c6", background: "rgba(70,68,72,0.55)" }}>{String(dados.titulo)}</div> : null}
      {folha?.f.linhas?.length ? <FolhaPDF key={folha.f.titulo} folha={folha.f} desde={folha.desde} t={t} /> : null}
    </AbsoluteFill>
  );
};

const UmaCena: React.FC<{ cena: Cena; roteiro: Roteiro }> = ({ cena, roteiro }) => {
  const e = estiloDe(roteiro.estilo);
  const Plana = cena.peca === "mapa" ? Mapa : cena.peca === "numero" ? Numero : cena.peca === "linha" ? Linha : cena.peca === "foto" ? Foto : null;   // pecas 2D/SVG (nao usam o palco 3D)
  const Peca = cena.peca === "barras" && cena.dados?.mesa ? Comparativo : ({ ...PECAS, moedas: Moedas, maquete: Maquete } as Record<string, React.FC<{ dados?: Record<string, unknown>; cor: string }>>)[cena.peca] ?? PECAS.barras;
  return (
    <AbsoluteFill>
      {Plana ? (
        <Plana dados={cena.dados} cor={e.destaque} texto={e.texto} fonte={e.fonte} />
      ) : (
        <Palco>
          <Peca dados={cena.dados} cor={e.destaque} />
        </Palco>
      )}
      {cena.peca === "maquete" ? <EtiquetaMaquete dados={cena.dados} fonte={e.fonte} /> : null}
      {cena.peca === "barras" && cena.dados?.titulo && !cena.dados?.mesa ? (   // achado do padrao Magno: faixa verde com o titulo ("EVENTOS x AJUDA AS FAMILIAS")
        <AbsoluteFill style={{ alignItems: "center", pointerEvents: "none" }}>
          <div style={{ marginTop: 330, fontFamily: "'Courier New', 'Consolas', monospace", fontWeight: 900, fontSize: 40, letterSpacing: 2, color: "#fff", background: "rgba(46,125,79,0.9)", borderRadius: 14, padding: "10px 26px", textAlign: "center", maxWidth: 900 }}>{String(cena.dados.titulo)}</div>
        </AbsoluteFill>
      ) : null}
      {cena.titulo && !roteiro.cabecalho ? <TituloDaCena texto={cena.titulo} cor={e.texto} fonte={e.fonte} destaque={e.destaque} /> : null}
    </AbsoluteFill>
  );
};

// Visual do PROMPT-OFICIAL-VIDEOS-MAGNO.md v2: pilula so com a cidade + selo pequeno do @ (foto no anel do Instagram) + cartoes de fonte e escala.
const MONO = "'JetBrains Mono', 'Consolas', 'Courier New', monospace";
const ANEL = "linear-gradient(45deg, #FEDA75, #FA7E1E, #D62976, #962FBF, #4F5BD5)";
const FOTO = "marca/eumagnoleite.jpg";

const SeloFoto: React.FC<{ tam: number }> = ({ tam }) => (
  <div style={{ width: tam, height: tam, borderRadius: "50%", background: ANEL, padding: tam * 0.07, flex: "none" }}>
    <img src={staticFile(FOTO)} style={{ width: "100%", height: "100%", borderRadius: "50%", objectFit: "cover", border: "2px solid #16151A" }} />
  </div>
);

const Cabecalho: React.FC<{ titulo: string; arroba?: string; fonte?: string; escala?: string; e: ReturnType<typeof estiloDe>; seloVisivel: boolean }> = ({ titulo, arroba, fonte, escala, e, seloVisivel }) => {
  const cartao = { fontFamily: MONO, fontWeight: 700 as const, letterSpacing: 1.3, color: "#F0EBE1", background: "#222127", border: "2px solid #3A3840", borderRadius: 11, padding: "8px 18px", textTransform: "uppercase" as const };
  return (
    <AbsoluteFill style={{ padding: "300px 90px 0", pointerEvents: "none" }}>{/* padrao Magno: cabecalho abaixo da barra do Instagram */}
      <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 14, height: 70, background: "#222127", border: "2px solid #3A3840", borderRadius: 35, padding: "0 26px 0 14px" }}>
          <div style={{ width: 40, height: 40, borderRadius: 20, background: "#EDE8DE", display: "flex", alignItems: "center", justifyContent: "center", flex: "none" }}>
            <div style={{ width: 12, height: 12, borderRadius: 6, background: "#2FA36B" }} />
          </div>
          <div style={{ fontFamily: MONO, fontWeight: 800, fontSize: 31, letterSpacing: 3, color: "#F0EBE1", textTransform: "uppercase" }}>{titulo}</div>
        </div>
        {arroba ? (
          <div style={{ display: "flex", alignItems: "center", gap: 12, opacity: seloVisivel ? 1 : 0 }}>
            <SeloFoto tam={56} />
            <div style={{ fontFamily: MONO, fontWeight: 700, fontSize: 26, color: "#F0EBE1" }}>{arroba}</div>
          </div>
        ) : null}
      </div>
      {fonte ? <div style={{ ...cartao, fontSize: fonte.length > 52 ? 14 : fonte.length > 42 ? 16 : 19, marginTop: 14, alignSelf: "flex-start" }}>{fonte}</div> : null}
      {escala ? <div style={{ ...cartao, fontSize: 16, fontWeight: 800, marginTop: 10, alignSelf: "flex-start" }}>{escala}</div> : null}
    </AbsoluteFill>
  );
};

// Selo GRANDE da abertura (0 a 3,5 s) e do fecho (ultimos 3,5 s): foto no anel + @ + frase, encolhe rumo ao cabecalho.
const SeloGrande: React.FC<{ arroba: string; frase: string; fonte: string }> = ({ arroba, frase }) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  const t = frame / fps, fimVideo = durationInFrames / fps;
  const abertura = t < 3.5;
  const fecho = t > fimVideo - 3.5;
  if (!abertura && !fecho) return null;
  // abertura: parado ate 3,0 s, depois 0,5 s encolhendo e subindo; fecho: cresce de volta ao meio
  const p = abertura ? interpolate(t, [3.0, 3.5], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }) : interpolate(t, [fimVideo - 3.5, fimVideo - 3.0], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const escala = interpolate(p, [0, 1], [1, 0.3]);
  const y = interpolate(p, [0, 1], [0, -760]);
  const op = abertura ? 1 : interpolate(t, [fimVideo - 0.9, fimVideo], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center", pointerEvents: "none" }}>
      <div style={{ transform: `translateY(${y}px) scale(${escala})`, opacity: op * (p < 1 ? 1 : 0), display: "flex", flexDirection: "column", alignItems: "center", gap: 18, background: "rgba(14,20,17,0.9)", borderRadius: 28, padding: "40px 70px" }}>
        <SeloFoto tam={140} />
        <div style={{ fontFamily: "Arial, sans-serif", fontWeight: 800, fontSize: 52, color: "#fff" }}>{arroba}</div>
        <div style={{ fontFamily: "Arial, sans-serif", fontWeight: 400, fontSize: 28, color: "rgb(150,220,180)" }}>{frase}</div>
      </div>
    </AbsoluteFill>
  );
};

// marca d'agua quicando como protetor de tela: vaivem linear, periodos diferentes (11 s / 16 s), opacidade 0,16
const MarcaDagua: React.FC<{ texto: string; fonte: string }> = ({ texto, fonte }) => {
  const t = useCurrentFrame() / FPS;
  const tri = (p: number) => { p %= 2; return p < 1 ? p : 2 - p; };
  const x = 20 + (1080 - 290) * tri((2 * t) / 11);
  const y = 80 + (1920 - 160) * tri((2 * t) / 16);
  return <div style={{ position: "absolute", left: x, top: y, fontFamily: fonte, fontWeight: 800, fontSize: 30, color: "#F0EBE1", opacity: 0.16, pointerEvents: "none" }}>{texto}</div>;
};

export const Estudio: React.FC<Roteiro> = (roteiro) => {
  const e = estiloDe(roteiro.estilo);
  return (
    <AbsoluteFill style={{ background: `linear-gradient(160deg, ${e.fundoA}, ${e.fundoB})` }}>
      {roteiro.cenas.map((c, i) => (
        <Sequence key={i} from={Math.round(c.inicio * FPS)} durationInFrames={Math.max(1, Math.round((c.fim - c.inicio) * FPS))}>
          <UmaCena cena={c} roteiro={roteiro} />
        </Sequence>
      ))}
      {roteiro.cabecalho ? <Cabecalho titulo={roteiro.cabecalho.titulo} arroba={roteiro.cabecalho.arroba} fonte={roteiro.cabecalho.fonte} escala={roteiro.cabecalho.escala} e={e} seloVisivel /> : null}
      {roteiro.cabecalho?.arroba ? <MarcaDagua texto={roteiro.cabecalho.arroba} fonte={e.fonte} /> : null}
      {roteiro.cabecalho?.arroba ? <SeloGrande arroba={roteiro.cabecalho.arroba} frase="Siga para ver a sua cidade" fonte={e.fonte} /> : null}
      <Legenda falas={roteiro.legenda} estilo={e} />
      {roteiro.audio ? <Audio src={staticFile(roteiro.audio)} /> : null}
    </AbsoluteFill>
  );
};

export const duracaoEmQuadros = (r: Roteiro): number => {
  const fim = Math.max(1, ...r.cenas.map((c) => c.fim), ...r.legenda.map((f) => f.fim));
  return Math.ceil(fim * FPS) + 15; // 0,5 s de respiro no final
};
