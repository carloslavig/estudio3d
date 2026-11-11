import React from "react";
import { Composition } from "remotion";
import { Estudio, duracaoEmQuadros } from "./Estudio";
import { ALTURA, FPS, LARGURA, type Roteiro } from "./tipos";
import exemplo from "../projetos/001-exemplo/roteiro.json";

// Uma unica composicao: o roteiro entra por --props=<arquivo.json>; a duracao vem das cenas e das falas.
export const Raiz: React.FC = () => (
  <Composition
    id="Estudio"
    component={Estudio as React.FC<Record<string, unknown>>}
    width={LARGURA}
    height={ALTURA}
    fps={FPS}
    durationInFrames={300}
    defaultProps={exemplo as unknown as Record<string, unknown>}
    calculateMetadata={({ props }) => ({ durationInFrames: duracaoEmQuadros(props as unknown as Roteiro) })}
  />
);
