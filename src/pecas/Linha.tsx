import React from "react";
import { AbsoluteFill, Easing, interpolate, useCurrentFrame, useVideoConfig } from "remotion";

// LINHA: grafico de linha minimo que se desenha sozinho, com um numero grande que conta junto da ponta.
// Ideia do /graph de artemnovitckii/claude-animation-skills (MIT), reescrita aqui em Remotion.
// dados: { valores: number[], rotulos?: string[], rotulo?: string, prefixo?, sufixo?, casas?, final? }
// "final" = numero em que a contagem termina (a contagem e o valor sob a ponta x final/ultimo valor); sem ele, conta nas unidades de "valores".
export const Linha: React.FC<{ dados?: Record<string, unknown>; cor: string; texto: string; fonte: string }> = ({ dados, cor, texto, fonte }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const valores = (Array.isArray(dados?.valores) ? (dados!.valores as unknown[]).filter((v): v is number => typeof v === "number") : [1, 3, 2, 5]).slice(0, 48);
  const rotulos = Array.isArray(dados?.rotulos) ? (dados!.rotulos as unknown[]).map(String) : [];
  const casas = typeof dados?.casas === "number" ? dados.casas : 0;
  const ultimo = valores[valores.length - 1] || 1;
  const fator = typeof dados?.final === "number" ? dados.final / ultimo : 1;
  const L = 900, A = 620, X0 = 60, Y0 = 40;
  const min = Math.min(0, ...valores), max = Math.max(...valores, 1e-9);
  const pontos = valores.map((v, i): [number, number] => [X0 + (i / Math.max(1, valores.length - 1)) * (L - X0 * 1.2), Y0 + (1 - (v - min) / (max - min)) * (A - Y0 * 2)]);
  const t = interpolate(frame, [8, Math.max(40, durationInFrames - 20)], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: Easing.inOut(Easing.cubic) });
  const pos = t * (valores.length - 1);
  const i0 = Math.min(valores.length - 2, Math.floor(pos)), f = pos - i0;
  const ponta: [number, number] = [pontos[i0][0] + (pontos[i0 + 1][0] - pontos[i0][0]) * f, pontos[i0][1] + (pontos[i0 + 1][1] - pontos[i0][1]) * f];
  const valorPonta = valores[i0] + (valores[i0 + 1] - valores[i0]) * f;
  const parte = [...pontos.slice(0, i0 + 1), ponta];
  const d = parte.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)} ${y.toFixed(1)}`).join(" ");
  const area = `${d} L${ponta[0].toFixed(1)} ${A - Y0} L${pontos[0][0].toFixed(1)} ${A - Y0} Z`;
  const numero = (valorPonta * fator).toLocaleString("pt-BR", { minimumFractionDigits: casas, maximumFractionDigits: casas });
  const grande = `${dados?.prefixo ?? ""}${numero}${dados?.sufixo ?? ""}`;
  const tam = grande.length > 12 ? 104 : grande.length > 9 ? 130 : 170;
  const passo = rotulos.length > 12 ? 2 : 1;
  const entrada = interpolate(frame, [0, 12], [0, 1], { extrapolateRight: "clamp" });
  return (
    <AbsoluteFill style={{ alignItems: "center", paddingTop: 400, fontFamily: fonte, color: texto, opacity: entrada }}>
      {dados?.rotulo ? <div style={{ fontSize: 44, opacity: 0.75, letterSpacing: 2 }}>{String(dados.rotulo)}</div> : null}
      <div style={{ fontSize: tam, lineHeight: 1.05, color: cor, textShadow: `0 0 50px ${cor}66`, fontVariantNumeric: "tabular-nums", marginBottom: 30 }}>{grande}</div>
      <svg width={L} height={A + 70} viewBox={`0 0 ${L} ${A + 70}`} style={{ overflow: "visible" }}>
        {[0, 0.25, 0.5, 0.75, 1].map((g) => (
          <line key={g} x1={X0} x2={L - X0 * 0.2} y1={Y0 + g * (A - Y0 * 2)} y2={Y0 + g * (A - Y0 * 2)} stroke="#ffffff" strokeOpacity={g === 1 ? 0.45 : 0.12} strokeWidth={2} />
        ))}
        <path d={area} fill={cor} fillOpacity={0.12} />
        <path d={d} fill="none" stroke={cor} strokeWidth={9} strokeLinecap="round" strokeLinejoin="round" style={{ filter: `drop-shadow(0 0 14px ${cor})` }} />
        <circle cx={ponta[0]} cy={ponta[1]} r={20 + 3 * Math.sin(frame * 0.3)} fill={cor} fillOpacity={0.3} />
        <circle cx={ponta[0]} cy={ponta[1]} r={11} fill="#ffffff" />
        {rotulos.map((r, i) => (i % passo === 0 && i < pontos.length ? <text key={i} x={pontos[i][0]} y={A + 24} fill={texto} fillOpacity={0.7} fontSize={28} textAnchor="middle" fontFamily={fonte}>{r}</text> : null))}
      </svg>
    </AbsoluteFill>
  );
};
