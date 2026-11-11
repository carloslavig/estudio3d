// Cada perfil/serie tem um estilo: cores e nome. Fontes do sistema (sem licenca de terceiros).
export type Estilo = {
  nome: string;
  fundoA: string;
  fundoB: string;
  destaque: string;
  texto: string;
  fonte: string;
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

export const estiloDe = (chave: string): Estilo => ESTILOS[chave] ?? ESTILOS.padrao;
