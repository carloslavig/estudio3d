import React, { useMemo } from "react";
import * as THREE from "three";
import { spring, useCurrentFrame, useVideoConfig } from "remotion";
import { corAno } from "./Maquete";

type Dados = Record<string, unknown>;
const num = (v: unknown, padrao: number) => (typeof v === "number" ? v : padrao);

// BARRAS: grafico de barras 3D que cresce em sequencia. dados.valores = numeros.
const quebrar = (g: CanvasRenderingContext2D, texto: string, larg: number): string[] => {
  const linhas: string[] = []; let atual = "";
  for (const w of texto.split(" ")) {
    const t = atual ? atual + " " + w : w;
    if (g.measureText(t).width > larg && atual) { linhas.push(atual); atual = w; } else atual = t;
  }
  if (atual) linhas.push(atual);
  return linhas.slice(0, 4);
};
const placaTexto = (texto: string, cor: string) => {
  const c = document.createElement("canvas"); c.width = 512; c.height = 256;
  const g = c.getContext("2d")!;
  g.font = "900 46px 'Courier New', monospace";
  const linhas = quebrar(g, texto, 440);
  g.fillStyle = "rgba(14,14,18,0.92)"; g.beginPath(); g.roundRect(6, 6, 500, 244, 30); g.fill();
  g.lineWidth = 7; g.strokeStyle = cor; g.stroke();
  g.textAlign = "center"; g.fillStyle = "#ffffff";
  const topo = 128 - (linhas.length * 52) / 2 + 40;
  linhas.forEach((l, i) => g.fillText(l, 256, topo + i * 52));
  const t = new THREE.CanvasTexture(c); t.anisotropy = 4; return t;
};

export const Barras: React.FC<{ dados?: Dados; cor: string }> = ({ dados, cor }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const valores = Array.isArray(dados?.valores) ? (dados!.valores as number[]) : [3, 5, 2, 6];
  const max = Math.max(...valores, 1);
  const rotulos = Array.isArray(dados?.rotulos) ? (dados!.rotulos as unknown[]).map(String) : [];
  // padrao Magno: TODA barra tem rotulo com o valor; sem rotulo, mostra o proprio numero
  const cores = Array.isArray(dados?.cores) ? (dados!.cores as unknown[]).map((c) => corAno(String(c), cor)) : [];
  const corDe = (i: number) => cores[i] ?? cor;   // cor de cada barra (ano/destaque do padrao Magno); sem cor, a do estilo
  const placas = useMemo(() => valores.map((v, i) => placaTexto(rotulos[i] ?? v.toLocaleString("pt-BR"), corDe(i))), [JSON.stringify(rotulos), JSON.stringify(valores), JSON.stringify(cores), cor]);
  const larg = 0.8;
  const folga = 0.3;
  const total = valores.length * (larg + folga) - folga;
  return (
    <group scale={Math.min(1, 4.4 / total)} rotation={[0.3, -0.5 + frame * 0.006, 0]} position={[0, dados?.titulo ? -0.6 : 0.3, 0]}>{/* com titulo (achado): desce para o rotulo da barra alta nao ficar sob a faixa */}
      {valores.map((v, i) => {
        const s = spring({ frame: frame - i * 6, fps, config: { damping: 14 } });
        const h = Math.max(0.02, (v / max) * (dados?.titulo ? 2.9 : 3.4) * s);
        return (
          <group key={i} position={[-total / 2 + larg / 2 + i * (larg + folga), 0, 0]}>
            <mesh position={[0, h / 2 - 1.7, 0]}>
              <boxGeometry args={[larg, h, larg]} />
              <meshStandardMaterial color={corDe(i)} roughness={0.35} metalness={0.25} />
            </mesh>
            {placas[i] ? (
              <mesh position={[0, h - 1.7 + 0.75, 0.6]} rotation={[0, -(-0.5 + frame * 0.006), 0]} scale={s}>
                <planeGeometry args={[Math.min(2.2, 1.0 + 8 / Math.max(valores.length, 3) * 0.28), 0.9]} />
                <meshBasicMaterial map={placas[i]!} transparent />
              </mesh>
            ) : null}
          </group>
        );
      })}
    </group>
  );
};

// GLOBO: esfera de arame com aneis girando. Bom para temas de mundo, rede, dados.
export const Globo: React.FC<{ dados?: Dados; cor: string }> = ({ cor }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const entrada = spring({ frame, fps, config: { damping: 16 } });
  return (
    <group scale={entrada * 0.85} rotation={[0.35, frame * 0.012, 0]} position={[0, 0.9, 0]}>
      <mesh>
        <sphereGeometry args={[1.7, 32, 24]} />
        <meshStandardMaterial color={cor} wireframe />
      </mesh>
      <mesh>
        <sphereGeometry args={[1.45, 32, 24]} />
        <meshStandardMaterial color="#0f172a" roughness={0.6} />
      </mesh>
      {[0, 1, 2].map((i) => (
        <mesh key={i} rotation={[Math.PI / 2 + i * 0.5, i * 0.7, frame * 0.02 * (i + 1)]}>
          <torusGeometry args={[2.2 + i * 0.25, 0.03, 12, 90]} />
          <meshStandardMaterial color={cor} emissive={cor} emissiveIntensity={0.6} />
        </mesh>
      ))}
    </group>
  );
};

// PILHA: caixas que caem e empilham. dados.itens = quantidade (1 a 12).
export const Pilha: React.FC<{ dados?: Dados; cor: string }> = ({ dados, cor }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const itens = Math.min(12, Math.max(1, Math.round(num(dados?.itens, 6))));
  return (
    <group rotation={[0.25, -0.45 + frame * 0.005, 0]} position={[0, 0.6, 0]}>
      {Array.from({ length: itens }, (_, i) => {
        const queda = spring({ frame: frame - i * 7, fps, config: { damping: 11 } });
        const y = -1.8 + i * 0.5 + (1 - queda) * 5;
        return (
          <mesh key={i} position={[Math.sin(i * 1.7) * 0.12, y, 0]} rotation={[0, i * 0.25, 0]}>
            <boxGeometry args={[2.2, 0.42, 1.6]} />
            <meshStandardMaterial color={cor} roughness={0.4} metalness={0.2} opacity={Math.min(1, queda * 2)} transparent />
          </mesh>
        );
      })}
    </group>
  );
};

export const PECAS = { barras: Barras, globo: Globo, pilha: Pilha } as const;
