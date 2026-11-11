import React from "react";
import { AbsoluteFill, useCurrentFrame, useVideoConfig } from "remotion";
import type { Fala } from "../tipos";
import type { Estilo } from "../estilos";

// Legenda grande na area segura inferior (evita a interface do Reels/TikTok/Shorts: ~260 px embaixo, 120 px nas laterais).
export const Legenda: React.FC<{ falas: Fala[]; estilo: Estilo }> = ({ falas, estilo }) => {
  const t = useCurrentFrame() / useVideoConfig().fps;
  const atual = falas.find((f) => t >= f.inicio && t < f.fim);
  if (!atual) return null;
  return (
    <AbsoluteFill style={{ justifyContent: "flex-end", alignItems: "center", padding: "0 120px 300px" }}>
      <div
        style={{
          fontFamily: estilo.fonte,
          fontSize: 68,
          lineHeight: 1.15,
          textAlign: "center",
          color: estilo.texto,
          textShadow: "0 4px 18px rgba(0,0,0,0.85)",
          background: "rgba(0,0,0,0.38)",
          padding: "16px 28px",
          borderRadius: 24,
        }}
      >
        {atual.texto}
      </div>
    </AbsoluteFill>
  );
};
