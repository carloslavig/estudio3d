import React from "react";
import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";

// FOTO: imagem real de acervo (public/fotos/<projeto>/N.jpg) com zoom e passeio lentos. dados: { arquivo: "fotos/...", credito?: "NASA/JPL" }
// Fundo = a mesma foto desfocada, para foto deitada nao deixar tarja preta no video em pe.
export const Foto: React.FC<{ dados?: Record<string, unknown>; cor: string; texto: string; fonte: string }> = ({ dados, cor, fonte }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();   // dentro da Sequence = duracao da cena
  const src = staticFile(String(dados?.arquivo ?? ""));
  const p = interpolate(frame, [0, Math.max(1, durationInFrames)], [0, 1], { extrapolateRight: "clamp" });
  const entrada = interpolate(frame, [0, 10], [0, 1], { extrapolateRight: "clamp" });
  return (
    <AbsoluteFill style={{ background: "#07080c" }}>
      <Img src={src} style={{ position: "absolute", width: "100%", height: "100%", objectFit: "cover", filter: "blur(38px) brightness(0.4)", transform: "scale(1.2)" }} />
      <div style={{ position: "absolute", left: 40, right: 40, top: 430, height: 960, overflow: "hidden", borderRadius: 28, border: `4px solid ${cor}`, boxShadow: "0 20px 60px rgba(0,0,0,0.6)", opacity: entrada }}>
        <Img src={src} style={{ width: "100%", height: "100%", objectFit: "cover", objectPosition: `${30 + 40 * p}% 50%`, transform: `scale(${1.02 + 0.1 * p})` }} />
        {dados?.credito ? (
          <div style={{ position: "absolute", left: 16, bottom: 14, maxWidth: 900, fontFamily: fonte, fontSize: 24, color: "#fff", background: "rgba(0,0,0,0.6)", borderRadius: 10, padding: "5px 14px", overflow: "hidden", whiteSpace: "nowrap", textOverflow: "ellipsis" }}>
            Imagem: {String(dados.credito)}
          </div>
        ) : null}
      </div>
    </AbsoluteFill>
  );
};
