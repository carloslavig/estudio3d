import React from "react";
import { AbsoluteFill, useCurrentFrame, useVideoConfig } from "remotion";
import type { Fala, Palavra } from "../tipos";
import type { Estilo } from "../estilos";

// Legenda karaoke na faixa de baixo (area segura do Reels). Regra do padrao Magno (PROMPT-OFICIAL v2):
// numeros NUNCA partem — "3" + ",7" vira "3,7"; "R$" gruda no numero. Palavra dita acende, as futuras ficam apagadas.
const fragmentoDeNumero = (anterior: string, atual: string): boolean =>
  /^[,.%]/.test(atual) && /[\d$]$/.test(anterior) || anterior === "R$";   // "Educação," + "2" e "2024," + "12" ficam separados (virgula com espaco)

export const juntarNumeros = (palavras: Palavra[]): Palavra[] => {
  const saida: Palavra[] = [];
  for (const p of palavras) {
    const ult = saida[saida.length - 1];
    if (ult && fragmentoDeNumero(ult.texto, p.texto)) {
      ult.texto += p.texto;
      ult.fim = p.fim;
    } else saida.push({ ...p });
  }
  return saida;
};

export const Legenda: React.FC<{ falas: Fala[]; estilo: Estilo }> = ({ falas, estilo }) => {
  const t = useCurrentFrame() / useVideoConfig().fps;
  const atual = falas.find((f) => t >= f.inicio && t < f.fim);
  if (!atual) return null;
  const palavras = atual.palavras && atual.palavras.length > 0 ? juntarNumeros(atual.palavras) : null;
  const karaoke = estilo.karaoke && palavras;
  return (
    <AbsoluteFill style={{ justifyContent: "flex-end", alignItems: "center", padding: "0 120px 300px" }}>
      <div
        style={{
          fontFamily: estilo.fonte,
          fontWeight: 700,
          fontSize: 56,
          letterSpacing: -2.9,
          lineHeight: 1.18,
          textAlign: "center",
          color: estilo.texto,
          background: "rgba(18,17,21,0.82)",
          boxShadow: "0 8px 30px rgba(0,0,0,0.45)",
          padding: "18px 30px",
          borderRadius: 18,
        }}
      >
        {karaoke
          ? palavras!.map((p, i) => {
              const dita = t >= p.inicio;
              return (
                <span key={i} style={{ color: dita ? (estilo.karaokeAtiva ?? estilo.texto) : "#7D7973", display: "inline-block", margin: "0 10px" }}>
                  {p.texto}
                </span>
              );
            })
          : atual.texto}
      </div>
    </AbsoluteFill>
  );
};
