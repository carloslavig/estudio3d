import React from "react";
import { AbsoluteFill, Audio, Sequence, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { Palco } from "./base/Palco";
import { Legenda } from "./base/Legenda";
import { PECAS } from "./pecas";
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

const UmaCena: React.FC<{ cena: Cena; roteiro: Roteiro }> = ({ cena, roteiro }) => {
  const e = estiloDe(roteiro.estilo);
  const Peca = PECAS[cena.peca] ?? PECAS.barras;
  return (
    <AbsoluteFill>
      <Palco>
        <Peca dados={cena.dados} cor={e.destaque} />
      </Palco>
      {cena.titulo ? <TituloDaCena texto={cena.titulo} cor={e.texto} fonte={e.fonte} destaque={e.destaque} /> : null}
    </AbsoluteFill>
  );
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
      <Legenda falas={roteiro.legenda} estilo={e} />
      {roteiro.audio ? <Audio src={staticFile(roteiro.audio)} /> : null}
    </AbsoluteFill>
  );
};

export const duracaoEmQuadros = (r: Roteiro): number => {
  const fim = Math.max(1, ...r.cenas.map((c) => c.fim), ...r.legenda.map((f) => f.fim));
  return Math.ceil(fim * FPS) + 15; // 0,5 s de respiro no final
};
