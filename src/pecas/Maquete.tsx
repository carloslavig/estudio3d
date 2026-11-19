import React, { useMemo } from "react";
import * as THREE from "three";
import { Easing, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";

// MAQUETE: mesa redonda bege com um COFRE no centro e uma CASINHA por setor (sobre um circulo verde). Quando o setor e falado, pilhas de moedas na cor do ano
// saem do cofre e voam ate a casinha; plaquinha pequena presa a pilha com o valor. Recriada so a partir da DESCRICAO do visual (nenhum codigo de terceiros).
// dados: { ano: "2025", cor?: "dourado"|"verde"|"rosa"|"#rrggbb", pilha_mi?: 2 (1 pilha = R$ x mi), titulo?: "ARRECADOU EM 2025: R$ 46,8 MI",
//          setores: [{ nome, valor (R$), rotulo?: "R$ 17,4 MI", tipo?: "escola"|"hospital"|"predio"|"rua"|"campo"|"casa", zerado?: boolean, t?: segundos desde o inicio da cena,
//                      anteriores?: [{ ano, rotulo, cor? }] }] }
export type SetorM = { nome: string; valor?: number; rotulo?: string; tipo?: string; zerado?: boolean; t?: number; anteriores?: { ano: string; rotulo: string; cor?: string }[] };
const CORES: Record<string, string> = { dourado: "#d4a537", verde: "#3aa676", rosa: "#d8707f", "rosa claro": "#efb3bd", "dourado claro": "#ead08f", "verde claro": "#9fd8b9", azul: "#4aa3ff", prata: "#cfd6e0", vermelho: "#ef4444" };
export const corAno = (c: string | undefined, padrao = "#d4a537") => (c && CORES[c] ? CORES[c] : c && /^#[0-9a-fA-F]{6}$/.test(c) ? c : padrao);

// instante (s) em que cada setor entra: o "t" dado, ou espalhado igualmente na cena
export const temposSetores = (setores: SetorM[], duracao: number): number[] => {
  const n = Math.max(1, setores.length);
  const passo = Math.max(1.2, (duracao - 3.5) / n);
  return setores.map((s, i) => (typeof s.t === "number" ? s.t : 1.2 + i * passo));
};
export const setorAtivo = (dados: Record<string, unknown> | undefined, t: number, duracao: number): SetorM | null => {
  const setores = (Array.isArray(dados?.setores) ? (dados!.setores as SetorM[]) : []).slice(0, 12);
  const ts = temposSetores(setores, duracao);
  let ativo: SetorM | null = null;
  setores.forEach((s, i) => { if (t >= ts[i]) ativo = s; });
  return ativo;
};

const placa = (linha1: string, linha2: string, cor: string) => {
  const c = document.createElement("canvas");
  c.width = 512; c.height = 190;
  const g = c.getContext("2d")!;
  g.fillStyle = cor;   // padrao Magno: caixinha PREENCHIDA na cor do ano, texto branco
  g.beginPath(); g.roundRect(6, 6, 500, 178, 26); g.fill();
  g.textAlign = "center"; g.fillStyle = "#ffffff";
  if (!linha2 && linha1.includes(" R$")) { const i = linha1.indexOf(" R$"); [linha1, linha2] = [linha1.slice(i + 1), linha1.slice(0, i)]; }   // achado: valor grande em cima, nome embaixo
  let tam = 74;
  do { g.font = "900 " + tam + "px Arial, sans-serif"; tam -= 4; } while (g.measureText(linha1).width > 470 && tam > 28);
  g.fillText(linha1, 256, 96);
  tam = 40;
  do { g.font = "700 " + tam + "px Arial, sans-serif"; tam -= 3; } while (g.measureText(linha2).width > 470 && tam > 20);
  g.globalAlpha = 0.85;
  g.fillText(linha2, 256, 158);
  const t = new THREE.CanvasTexture(c);
  t.anisotropy = 4;
  return t;
};

const Casinha: React.FC<{ tipo: string; zerado: boolean }> = ({ tipo, zerado }) => {
  const parede = zerado ? "#8a8a8a" : tipo === "hospital" ? "#f1f5f9" : tipo === "escola" ? "#e8c27a" : tipo === "predio" ? "#d9d3c4" : "#c9bba0";
  const telhado = zerado ? "#6a6a6a" : tipo === "hospital" ? "#ef4444" : "#a8452f";
  if (tipo === "campo") {
    return (
      <group>
        <mesh position={[0, 0.06, 0]}><boxGeometry args={[1.25, 0.05, 0.8]} /><meshStandardMaterial color={zerado ? "#8a8a8a" : "#2f9e57"} /></mesh>
        <mesh position={[0, 0.1, 0]}><boxGeometry args={[0.02, 0.02, 0.8]} /><meshStandardMaterial color="#ffffff" /></mesh>
        {[-0.62, 0.62].map((x) => <mesh key={x} position={[x, 0.3, 0]}><boxGeometry args={[0.04, 0.4, 0.3]} /><meshStandardMaterial color="#ffffff" /></mesh>)}
      </group>
    );
  }
  if (tipo === "rua") {
    return (
      <group>
        <mesh position={[0, 0.06, 0]}><boxGeometry args={[1.3, 0.04, 0.5]} /><meshStandardMaterial color={zerado ? "#777" : "#3a3f47"} /></mesh>
        <mesh position={[-0.2, 0.28, 0]}><boxGeometry args={[0.55, 0.3, 0.32]} /><meshStandardMaterial color={zerado ? "#999" : "#f59e0b"} /></mesh>
        <mesh position={[0.2, 0.22, 0]}><boxGeometry args={[0.25, 0.2, 0.3]} /><meshStandardMaterial color={zerado ? "#999" : "#2563eb"} /></mesh>
      </group>
    );
  }
  return (
    <group>
      <mesh position={[0, 0.38, 0]}><boxGeometry args={[0.9, 0.62, 0.7]} /><meshStandardMaterial color={parede} roughness={0.6} /></mesh>
      <mesh position={[0, 0.9, 0]} rotation={[0, Math.PI / 4, 0]}><coneGeometry args={[0.72, 0.42, 4]} /><meshStandardMaterial color={telhado} roughness={0.55} /></mesh>
      {tipo === "predio" ? [-0.3, -0.1, 0.1, 0.3].map((x) => <mesh key={x} position={[x, 0.36, 0.38]}><cylinderGeometry args={[0.045, 0.045, 0.58, 12]} /><meshStandardMaterial color="#f5f5f4" /></mesh>) : null}
      {tipo === "hospital" ? <>
        <mesh position={[0, 0.5, 0.36]}><boxGeometry args={[0.36, 0.1, 0.02]} /><meshStandardMaterial color="#ef4444" /></mesh>
        <mesh position={[0, 0.5, 0.36]}><boxGeometry args={[0.1, 0.36, 0.02]} /><meshStandardMaterial color="#ef4444" /></mesh></> : null}
      {tipo === "escola" ? <>
        <mesh position={[0.34, 1.0, 0.2]}><boxGeometry args={[0.025, 0.5, 0.025]} /><meshStandardMaterial color="#cbd5e1" /></mesh>
        <mesh position={[0.43, 1.18, 0.2]}><boxGeometry args={[0.18, 0.1, 0.01]} /><meshStandardMaterial color="#22c55e" /></mesh></> : null}
      <mesh position={[0, 0.2, 0.355]}><boxGeometry args={[0.16, 0.3, 0.01]} /><meshStandardMaterial color="#5b4636" /></mesh>
    </group>
  );
};

// MESA CONTINUA do padrao Magno (PROMPT-OFICIAL secao 5). Dados extras montados pelo Braniac (link/render_estudio.py):
//  todos: [{nome,tipo}] TODOS os setores do video na ordem da fala (posicao fixa no anel), ja: nomes falados antes desta cena (casinha ja de pe),
//  ja_ordem: os mesmos na ordem da fala, ja_ano: {nome: valor} falados antes no mesmo ano (pilhas ja pousadas), gastou_ano (R$) para as pilhas do cofre,
//  t0: inicio da cena no video (camera usa o tempo GLOBAL e gira num sentido so), volta: indice do ano (0,1,2), inicio_ano: a camera desce da vista de cima.
type Todo = { nome: string; tipo?: string };
export const Maquete: React.FC<{ dados?: Record<string, unknown>; cor: string; children?: React.ReactNode }> = ({ dados, children }) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  const t = frame / fps;
  const setores = ((Array.isArray(dados?.setores) ? (dados!.setores as SetorM[]) : []) as SetorM[]).slice(0, 16);
  const todos: Todo[] = Array.isArray(dados?.todos) && (dados!.todos as Todo[]).length ? (dados!.todos as Todo[]).slice(0, 16) : setores.map((x) => ({ nome: x.nome, tipo: x.tipo }));
  const ja = new Set(Array.isArray(dados?.ja) ? (dados!.ja as string[]) : []);
  const jaAno = (dados?.ja_ano ?? {}) as Record<string, number>;
  const anosAnt = (dados?.anos_ant ?? {}) as Record<string, { cor: string; valor: number }[]>;
  const corDoAno = corAno(typeof dados?.cor === "string" ? (dados!.cor as string) : undefined);
  const ano = String(dados?.ano ?? "");
  const pilhaMi = typeof dados?.pilha_mi === "number" && dados.pilha_mi > 0 ? dados.pilha_mi : 2;
  const tempos = temposSetores(setores, durationInFrames / fps);
  const geral = !!dados?.visao_geral;
  const iAtivo = (() => { let k = -1; tempos.forEach((tt, j) => { if (t >= tt) k = j; }); return k; })();
  const N = Math.max(1, todos.length);
  const angDe = (nome: string) => { const i = Math.max(0, todos.findIndex((x) => x.nome === nome)); return (i / N) * Math.PI * 2 - Math.PI / 2; };
  const placas = useMemo(
    () => setores.map((x) => [placa(x.rotulo ?? "", ano, corDoAno), ...(x.anteriores ?? []).slice(0, 2).map((a) => placa(a.rotulo, a.ano, corAno(a.cor)))]),
    [JSON.stringify(setores), ano, corDoAno],
  );
  const saldo = dados?.saldo as { tipo?: string; rotulo?: string; valor?: number; t?: number } | undefined;
  const aMais = saldo?.tipo === "a_mais";
  const placaSaldo = useMemo(() => (saldo?.rotulo ? placa(saldo.rotulo.replace(/^(GASTOU|SOBRARAM|SOBROU)\s+/i, "").replace(/\s+A MAIS$/i, ""), aMais ? "A MAIS" : "SOBROU", aMais ? "#d64545" : "#2fa36b") : null), [saldo?.rotulo, aMais]);
  // camera: vira para o setor falado, sempre no mesmo sentido (+2pi por ano), deriva lenta no tempo GLOBAL; na virada de ano desce da vista de cima
  const T = (typeof dados?.t0 === "number" ? (dados!.t0 as number) : 0) + t;
  const volta = typeof dados?.volta === "number" ? (dados!.volta as number) : 0;
  const ordemJa = Array.isArray(dados?.ja_ordem) ? (dados!.ja_ordem as string[]).filter((x) => x in jaAno) : [];
  const alvo = (nome: string) => angDe(nome) + Math.PI / 2 + volta * Math.PI * 2;   // setor falado vai para o FUNDO da mesa: a camera olha por cima do cofre (padrao Magno)
  let yaw = ordemJa.length ? alvo(ordemJa[ordemJa.length - 1]) : setores.length ? alvo(setores[0].nome) - 0.5 : volta * Math.PI * 2;
  setores.forEach((x, i) => {
    const k = interpolate(t, [tempos[i] - 0.2, tempos[i] + 1.0], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: Easing.inOut(Easing.cubic) });
    yaw = yaw + (Math.max(yaw, alvo(x.nome)) - yaw) * k;
  });
  yaw += 0.006 * Math.min(t, 4);   // respiro leve so no comeco da cena (deriva global acumulava e tirava o setor do centro)
  const pitch = geral ? 1.05 : dados?.inicio_ano ? interpolate(t, [0, 2.5], [1.35, 0.62], { extrapolateRight: "clamp", easing: Easing.inOut(Easing.cubic) }) : 0.62;
  const TOPO = -1.3, RAIO_ANEL = 5.0;
  // aproxima do setor falado (a frente da mesa, onde a camera ja o trouxe); abre so na virada de ano, no fecho e nos achados.
  // Setor no fundo, pitch 0,75, escala 1: deslocar (0, -2,6, +4) poe a casinha no centro da tela e o cofre na beira de baixo (conta feita no papel).
  const kZoom = geral || !setores.length ? 0 : !dados?.inicio_ano ? 1 : interpolate(t, [tempos[0] - 0.3, tempos[0] + 1.2], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: Easing.inOut(Easing.cubic) });   // casinhas espalhadas pela borda da mesa, pequenas (padrao Magno)
  // pilhas em volta do cofre: o ano comeca empilhado e diminui conforme os setores sao falados
  const gastouAno = typeof dados?.gastou_ano === "number" ? (dados!.gastou_ano as number) : 0;
  const falado = Object.values(jaAno).reduce((a, b) => a + b, 0) + setores.reduce((a, x, i) => a + (t >= tempos[i] + 0.4 && typeof x.valor === "number" ? x.valor : 0), 0);
  const noCofre = gastouAno > 0 && !geral ? Math.round(Math.min(30, gastouAno / (pilhaMi * 1e6)) * Math.max(0, 1 - falado / gastouAno)) : 0;
  return (
    <group position={[0, -0.2 - 1.8 * kZoom, 2.6 * kZoom]} rotation={[pitch + 0.18 * kZoom, yaw, 0]} scale={(geral ? 0.42 : 0.62) + 0.23 * kZoom}>{/* setor no centro da tela, cofre na beira de baixo */}{/* kZoom: camera casinha por casinha (padrao Magno) */}
      <mesh position={[0, TOPO - 0.2, 0]}><cylinderGeometry args={[6.8, 6.8, 0.4, 96]} /><meshStandardMaterial color="#c6b497" roughness={0.75} /></mesh>
      <mesh position={[0, TOPO - 0.42, 0]}><cylinderGeometry args={[6.95, 6.95, 0.08, 96]} /><meshStandardMaterial color="#6b4a2e" roughness={0.6} /></mesh>
      {[0.3, 1.2, 2.1, 2.9, 3.8, 4.7, 5.6].map((a, i) => (
        <mesh key={i} position={[Math.cos(a) * (5.3 + (i % 2) * 0.6), TOPO + 0.005, Math.sin(a) * (5.3 + (i % 2) * 0.6)]}>
          <cylinderGeometry args={[0.7 + (i % 3) * 0.25, 0.7 + (i % 3) * 0.25, 0.01, 28]} /><meshStandardMaterial color="#93a463" roughness={0.95} transparent opacity={0.55} />
        </mesh>
      ))}
      <group position={[0, TOPO, 0]}>
        <mesh position={[0, 0.6, 0]}><boxGeometry args={[1.3, 1.2, 1.2]} /><meshStandardMaterial color="#4b5563" roughness={0.4} metalness={0.5} /></mesh>
        <mesh position={[0, 0.6, 0.61]} rotation={[Math.PI / 2, 0, 0]}><cylinderGeometry args={[0.3, 0.3, 0.05, 28]} /><meshStandardMaterial color={corDoAno} emissive={corDoAno} emissiveIntensity={0.3} metalness={0.4} roughness={0.4} /></mesh>
      </group>
      {Array.from({ length: noCofre }, (_, k) => {
        const a = (k / Math.max(Math.min(noCofre, 12), 1)) * Math.PI * 2 + (k >= 12 ? 0.26 : 0), r = k >= 12 ? 1.45 : 1.1;
        return <PilhaMoedas key={"c" + k} pos={[Math.cos(a) * r, TOPO, Math.sin(a) * r]} cor={corDoAno} alt={0.38} visivel />;
      })}
      {saldo && placaSaldo && t >= (saldo.t ?? 0) ? (
        <group position={[1.9, TOPO, 0.5]}>
          {Array.from({ length: Math.ceil(Math.max(1, Math.min(8, Math.round((saldo.valor ?? 0) / (pilhaMi * 1e6)) || 1)) * Math.min(1, (t - (saldo.t ?? 0)) / 0.8)) }, (_, k) => (
            <PilhaMoedas key={k} pos={[0, k * 0.4, 0]} cor={aMais ? "#d64545" : "#2fa36b"} alt={0.38} visivel />
          ))}
          <mesh position={[0, Math.max(1, Math.min(8, Math.round((saldo.valor ?? 0) / (pilhaMi * 1e6)) || 1)) * 0.4 + 0.5, 0]} rotation={[0, -yaw, 0]}>
            <planeGeometry args={[1.5, 0.56]} /><meshBasicMaterial map={placaSaldo} transparent />
          </mesh>
        </group>
      ) : null}
      {todos.map((todo, gi) => {
        const ang = angDe(todo.nome);
        const x = Math.cos(ang) * RAIO_ANEL, z = Math.sin(ang) * RAIO_ANEL;
        const i = setores.findIndex((x2) => x2.nome === todo.nome);
        const s = i >= 0 ? setores[i] : null;
        const jaDePe = ja.has(todo.nome) || geral;
        if (!s && !jaDePe) return null;
        const f0 = s ? tempos[i] * fps : -999;
        const nasce = jaDePe ? 1 : spring({ frame: frame - f0, fps, config: { damping: 14 } });
        const valor = s?.valor ?? jaAno[todo.nome];
        const zerado = s ? !!s.zerado || !(typeof s.valor === "number" && s.valor > 0) : false;
        const pilhas = typeof valor === "number" && valor > 0 && (s || todo.nome in jaAno) ? Math.max(1, Math.min(18, Math.round(valor / (pilhaMi * 1e6)))) : 0;
        const voo = (k: number) => (!s ? 1 : Math.min(1, Math.max(0, (frame - f0 - 6 - k * 2) / (0.9 * fps))));
        return (
          <group key={gi}>
            <group position={[x, TOPO + 0.03, z]} scale={Math.max(0.001, nasce)}>
              <mesh position={[0, 0.02, 0]}><cylinderGeometry args={[0.62, 0.62, 0.04, 40]} /><meshStandardMaterial color={zerado ? "#8a8a8a" : "#2e7d4f"} roughness={0.7} /></mesh>
              <group rotation={[0, -ang + Math.PI / 2, 0]} scale={1.25}><Casinha tipo={(s?.tipo ?? todo.tipo) ?? "casa"} zerado={zerado} /></group>
            </group>
            {Array.from({ length: pilhas }, (_, k) => {
              const p = voo(k);
              const col = k % 4, lin = Math.floor(k / 4), lado = (col - 1.5) * 0.27;   // bloco compacto 4 por fileira na frente da casinha (nada de faixa de moedas)
              const r = (RAIO_ANEL - 0.95 - lin * 0.27) / RAIO_ANEL;
              const alvoX = x * r + Math.cos(ang + Math.PI / 2) * lado, alvoZ = z * r + Math.sin(ang + Math.PI / 2) * lado;
              const px = alvoX * p, pz = alvoZ * p, py = TOPO + 0.15 + Math.sin(p * Math.PI) * 1.4;
              return <PilhaMoedas key={k} pos={[px, py, pz]} cor={corDoAno} alt={0.34 + 0.06 * (k % 3)} visivel={p > 0} />;
            })}
            {(anosAnt[todo.nome] ?? []).map((h, j) => {   // regra do Magno: pilhas de cada ano JUNTAS, lado a lado, na frente da casinha
              const q = Math.max(1, Math.min(5, Math.round(h.valor / (pilhaMi * 1e6))));
              const lado = -0.75 - j * 0.6;   // ao lado do bloco do ano atual, um bloquinho por ano anterior
              const r = (RAIO_ANEL - 0.95) / RAIO_ANEL;
              return Array.from({ length: q }, (_, k) => (
                <PilhaMoedas key={"a" + j + "-" + k} pos={[x * (r - Math.floor(k / 2) * 0.27 / RAIO_ANEL) + Math.cos(ang + Math.PI / 2) * (lado - (k % 2) * 0.27), TOPO, z * (r - Math.floor(k / 2) * 0.27 / RAIO_ANEL) + Math.sin(ang + Math.PI / 2) * (lado - (k % 2) * 0.27)]} cor={corAno(h.cor)} alt={0.36} visivel />
              ));
            })}
            {s && !zerado && !geral && i === iAtivo ? (placas[i] ?? []).map((tex, j) => {   // so o setor falado mostra plaquinha (nunca uma sobre a outra)
              const pop = spring({ frame: frame - f0 - 14 - j * 8, fps, config: { damping: 12 } });
              return (
                <mesh key={j} position={[x * 0.82, TOPO + 1.35 + j * 0.52, z * 0.82]} rotation={[0, -yaw, 0]} scale={pop * (j === 0 ? 1 : 0.8)}>
                  <planeGeometry args={[1.5, 0.56]} />
                  <meshBasicMaterial map={tex} transparent />
                </mesh>
              );
            }) : null}
          </group>
        );
      })}
      {children ? <group position={[0, TOPO, 0]} rotation={[0, -yaw, 0]}>{children}</group> : null}
    </group>
  );
};

// pilha de moedas: cilindro com serrilha desenhada (uma textura por cor, reaproveitada)
const serrilhas: Record<string, THREE.CanvasTexture> = {};
const serrilha = (cor: string) => {
  if (serrilhas[cor]) return serrilhas[cor];
  const c = document.createElement("canvas"); c.width = 64; c.height = 128;
  const g = c.getContext("2d")!;
  g.fillStyle = cor; g.fillRect(0, 0, 64, 128);
  g.fillStyle = "rgba(0,0,0,0.28)";
  for (let y = 0; y < 128; y += 16) g.fillRect(0, y, 64, 3);
  const t = new THREE.CanvasTexture(c); t.wrapS = t.wrapT = THREE.RepeatWrapping;
  return (serrilhas[cor] = t);
};
export const PilhaMoedas: React.FC<{ pos: [number, number, number]; cor: string; alt: number; visivel: boolean }> = ({ pos, cor, alt, visivel }) => (
  <group position={pos} scale={visivel ? 1 : 0.001}>
    <mesh position={[0, alt / 2, 0]}><cylinderGeometry args={[0.13, 0.13, alt, 22]} /><meshStandardMaterial map={serrilha(cor)} metalness={0.45} roughness={0.35} /></mesh>
    <mesh position={[0, alt + 0.005, 0]}><cylinderGeometry args={[0.128, 0.128, 0.01, 22]} /><meshStandardMaterial color={cor} emissive={cor} emissiveIntensity={0.25} metalness={0.5} roughness={0.3} /></mesh>
  </group>
);

// ACHADO do padrao Magno: a mesma mesa, um TAPETE verde no meio com pilhas de moedas lado a lado (uma coluna por valor),
// plaquinha preenchida em cada uma e o titulo escrito na borda do tapete. dados: valores, rotulos, cores, titulo, barras[].t (s da cena)
export const Comparativo: React.FC<{ dados?: Record<string, unknown>; cor: string }> = ({ dados, cor }) => (
  <Maquete dados={{ ...dados, setores: [], saldo: undefined }} cor={cor}><Gaveta dados={dados} cor={cor} /></Maquete>
);
const Gaveta: React.FC<{ dados?: Record<string, unknown>; cor: string }> = ({ dados, cor }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const valores = Array.isArray(dados?.valores) ? (dados!.valores as number[]) : [1, 2];
  const rotulos = Array.isArray(dados?.rotulos) ? (dados!.rotulos as unknown[]).map(String) : [];
  const cores = Array.isArray(dados?.cores) ? (dados!.cores as unknown[]).map((c) => corAno(String(c), cor)) : [];
  const ts = Array.isArray(dados?.barras) ? (dados!.barras as { t?: number }[]).map((b) => b?.t) : [];
  const max = Math.max(...valores, 1), n = valores.length;
  const largura = Math.min(5.2, 1.0 * n + 0.6), passo = largura / n;
  const titulo = String(dados?.titulo ?? "");
  const placas = useMemo(() => valores.map((v, i) => placa(rotulos[i] ?? v.toLocaleString("pt-BR"), "", cores[i] ?? cor)), [JSON.stringify(rotulos), JSON.stringify(valores), JSON.stringify(cores), cor]);
  const faixa = useMemo(() => {
    const c = document.createElement("canvas"); c.width = 1024; c.height = 96;
    const g = c.getContext("2d")!; g.fillStyle = "#1f6b40"; g.fillRect(0, 0, 1024, 96);
    g.fillStyle = "#ffffff"; g.textAlign = "center"; g.font = "900 " + (titulo.length > 30 ? 40 : 52) + "px 'Courier New', monospace";
    g.fillText(titulo.toUpperCase(), 512, 66);
    return new THREE.CanvasTexture(c);
  }, [titulo]);
  return (
    <group position={[0, 1.27, 1.3]} scale={n > 3 ? 0.95 : 1.2}>{/* gaveta no miolo da mesa, entre o cofre e a camera */}
      <mesh position={[0, -1.29, 0.2]}><boxGeometry args={[Math.max(4.4, largura + 0.6), 0.03, 2.0]} /><meshStandardMaterial color="#2e7d4f" roughness={0.8} /></mesh>
      <mesh position={[0, -1.27, 1.21]} rotation={[-Math.PI / 2, 0, 0]}><planeGeometry args={[Math.max(4.4, largura + 0.6), 0.44]} /><meshBasicMaterial map={faixa} /></mesh>
      {valores.map((v, i) => {
        const f0 = (typeof ts[i] === "number" ? (ts[i] as number) : 0.3 + i * 0.8) * fps;
        const nasce = spring({ frame: frame - f0, fps, config: { damping: 14 } });
        const qtd = Math.max(1, Math.round((v / max) * 12));
        const x = -largura / 2 + passo / 2 + i * passo;
        const c = cores[i] ?? cor;
        return (
          <group key={i} position={[x, -1.27, 0.1]}>
            {Array.from({ length: qtd }, (_, k) => {
              const col = k % 3, lin = Math.floor(k / 3);
              const alto = Math.min(1, Math.max(0, nasce * qtd - k));
              return <PilhaMoedas key={k} pos={[(col - 1) * 0.29, 0, (lin - 1.5) * 0.29]} cor={c} alt={0.42 * alto + 0.001} visivel={alto > 0} />;
            })}
            <mesh position={[0, 1.15 + (n > 3 ? (i % 2) * 0.62 : 0), 0]} rotation={[0, 0, 0]} scale={nasce}>{/* muitas colunas: plaquinhas em dois andares para nao encavalar */}
              <planeGeometry args={[Math.min(1.9, Math.max(1.45, passo * 1.05)), Math.min(1.9, Math.max(1.45, passo * 1.05)) * 0.37]} />
              <meshBasicMaterial map={placas[i]} transparent />
            </mesh>
          </group>
        );
      })}
    </group>
  );
};
