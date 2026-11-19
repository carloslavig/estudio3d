// Formato do roteiro de um video (projetos/<nome>/roteiro.json). Tudo em SEGUNDOS.
export type Palavra = { inicio: number; fim: number; texto: string };
export type Fala = { inicio: number; fim: number; texto: string; palavras?: Palavra[] };

export type Cena = {
  inicio: number;
  fim: number;
  titulo?: string;
  // peca 3D da biblioteca (src/pecas): "barras" | "globo" | "pilha"
  peca: "barras" | "globo" | "pilha" | "mapa" | "numero" | "linha" | "moedas" | "maquete" | "foto";   // foto {arquivo: "fotos/<projeto>/1.jpg", credito?}
  // dados da peca: barras {valores:number[]}, pilha {itens:number}, globo {}, mapa {uf,municipio,zoom_final?}, numero {valor,prefixo?,sufixo?,rotulo?,detalhe?,casas?}, linha {valores,rotulos?,rotulo?,prefixo?,sufixo?,casas?,final?}, moedas {pilhas:[{ano,rotulo?,valor?,itens?,cor?}]}, maquete {ano,cor?,pilha_mi?,titulo?,setores:[{nome,valor,rotulo?,tipo?,zerado?,t?,anteriores?}]}
  dados?: Record<string, unknown>;
};

export type Cabecalho = { titulo: string; arroba?: string; fonte?: string; escala?: string }; // fonte e escala: linhas miudas sob o titulo // faixa fixa no topo de todo o video (ex.: nome da cidade + @do canal)

export type Roteiro = {
  cabecalho?: Cabecalho;
  titulo: string;
  estilo: string; // chave de src/estilos.ts
  audio?: string; // arquivo dentro de public/ (ex.: "audio/001.wav")
  legenda: Fala[]; // gerada por tools/transcrever.py a partir da narracao
  cenas: Cena[];
};

export const FPS = 30;
export const LARGURA = 1080;
export const ALTURA = 1920;
