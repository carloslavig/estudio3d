import React from "react";
import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";

// NUMERO: numero grande com rotulo e detalhe. dados: { valor: 12400000 | "R$ 12,4 mi", prefixo?, sufixo?, rotulo?, detalhe?, casas? }
// Se "valor" for numero, ele conta de zero ate o valor; se for texto, aparece como esta.
export const Numero: React.FC<{ dados?: Record<string, unknown>; cor: string; texto: string; fonte: string }> = ({ dados, cor, texto, fonte }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const entrada = spring({ frame, fps, config: { damping: 14 } });
  const valor = dados?.valor;
  const casas = typeof dados?.casas === "number" ? dados.casas : 0;
  const conta = interpolate(frame, [4, 40], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const exibido = typeof valor === "number" ? (valor * conta).toLocaleString("pt-BR", { minimumFractionDigits: casas, maximumFractionDigits: casas }) : String(valor ?? "");
  const grande = `${dados?.prefixo ?? ""}${exibido}${dados?.sufixo ?? ""}`;
  const tamanho = grande.length > 12 ? 118 : grande.length > 8 ? 150 : 190;
  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center", padding: "0 80px", fontFamily: fonte, color: texto }}>
      <div style={{ transform: `scale(${0.6 + 0.4 * entrada})`, opacity: entrada, textAlign: "center" }}>
        <div style={{ fontSize: tamanho, lineHeight: 1, color: cor, textShadow: `0 0 60px ${cor}77`, fontVariantNumeric: "tabular-nums" }}>{grande}</div>
        {dados?.rotulo ? <div style={{ fontSize: 58, marginTop: 34, opacity: 0.95 }}>{String(dados.rotulo)}</div> : null}
        {dados?.detalhe ? <div style={{ fontSize: 40, marginTop: 20, opacity: 0.7 }}>{String(dados.detalhe)}</div> : null}
      </div>
    </AbsoluteFill>
  );
};
