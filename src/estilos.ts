// Cada perfil/serie tem um estilo: cores e nome. Fontes do sistema (sem licenca de terceiros).
export type Estilo = {
  nome: string;
  fundoA: string;
  fundoB: string;
  destaque: string;
  texto: string;
  fonte: string;
  karaokeAtiva?: string; // cor da palavra da hora (padrao: destaque); as proximas ficam cinza
  karaoke?: boolean; // legenda palavra a palavra, com a palavra da hora destacada
};

export const ESTILOS: Record<string, Estilo> = {
  padrao: {
    nome: "Explicando em 3D",
    fundoA: "#0b1020",
    fundoB: "#1b2a5a",
    destaque: "#38bdf8",
    texto: "#f8fafc",
    fonte: "'Arial Black', 'Segoe UI', system-ui, sans-serif",
  },
  quente: {
    nome: "Explicando em 3D",
    fundoA: "#1f0a0a",
    fundoB: "#5a1b1b",
    destaque: "#fbbf24",
    texto: "#fff7ed",
    fonte: "'Arial Black', 'Segoe UI', system-ui, sans-serif",
  },
};

// Paleta exata do PROMPT-OFICIAL-VIDEOS-MAGNO.md v2 (12/11/2026)
ESTILOS.mesa = {
  nome: "Mesa de moedas",
  fundoA: "#2a221b", // padrao Magno: fundo escuro quente (vinheta marrom) em volta da mesa
  fundoB: "#0d0b0a",
  destaque: "#f4c542",
  texto: "#F0EBE1",
  fonte: "'Inter', 'Segoe UI', system-ui, sans-serif", // shortcut: Inter nao instalada no PC, cai no Segoe UI; instalar Inter/JetBrains Mono quando der
  karaoke: true,
  karaokeAtiva: "#F0EBE1",
};

export const estiloDe = (chave: string): Estilo => ESTILOS[chave] ?? ESTILOS.padrao;
