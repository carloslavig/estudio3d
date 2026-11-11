// Formato do roteiro de um video (projetos/<nome>/roteiro.json). Tudo em SEGUNDOS.
export type Fala = { inicio: number; fim: number; texto: string };

export type Cena = {
  inicio: number;
  fim: number;
  titulo?: string;
  // peca 3D da biblioteca (src/pecas): "barras" | "globo" | "pilha"
  peca: "barras" | "globo" | "pilha";
  // dados da peca: barras {valores:number[]}, pilha {itens:number}, globo {}
  dados?: Record<string, unknown>;
};

export type Roteiro = {
  titulo: string;
  estilo: string; // chave de src/estilos.ts
  audio?: string; // arquivo dentro de public/ (ex.: "audio/001.wav")
  legenda: Fala[]; // gerada por tools/transcrever.py a partir da narracao
  cenas: Cena[];
};

export const FPS = 30;
export const LARGURA = 1080;
export const ALTURA = 1920;
