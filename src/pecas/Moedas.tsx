import React, { useMemo } from "react";
import * as THREE from "three";
import { spring, useCurrentFrame, useVideoConfig } from "remotion";

// MOEDAS: mesa redonda bege com pilhas de moedas (uma por ano), cada pilha com uma plaquinha presa (ano + valor). Inspirado so na DESCRICAO do visual pedido
// (nenhum codigo de terceiros). dados: { pilhas: [{ ano: "2024", rotulo?: "R$ 11,3 bi", valor?: number, itens?: number (1 a 40), cor?: "dourado"|"verde"|"rosa"|"azul"|"#rrggbb" }] }
// Altura da pilha: "itens" se vier; senao proporcional a "valor" (a maior vira 26 moedas); senao 10.
type Pilha = { ano?: string; rotulo?: string; valor?: number; itens?: number; cor?: string };
const CORES: Record<string, string> = { dourado: "#f4c542", verde: "#3fbf6f", rosa: "#ff6fa8", azul: "#4aa3ff", prata: "#cfd6e0", vermelho: "#ef4444" };
const corDe = (c: string | undefined, i: number) => (c && CORES[c] ? CORES[c] : c && /^#[0-9a-fA-F]{6}$/.test(c) ? c : Object.values(CORES)[i % 3]);

const placa = (ano: string, rotulo: string, cor: string) => {
  const c = document.createElement("canvas");
  c.width = 512; c.height = 220;
  const g = c.getContext("2d")!;
  g.fillStyle = "rgba(20,20,28,0.94)";
  g.beginPath(); g.roundRect(6, 6, 500, 208, 36); g.fill();
  g.lineWidth = 8; g.strokeStyle = cor; g.stroke();
  g.fillStyle = cor; g.font = "900 64px 'Arial Black', Arial, sans-serif"; g.textAlign = "center";
  g.fillText(ano, 256, 82);
  g.fillStyle = "#fffaf0"; g.font = "900 " + (rotulo.length > 11 ? 52 : 66) + "px 'Arial Black', Arial, sans-serif";
  g.fillText(rotulo, 256, 168);
  const t = new THREE.CanvasTexture(c);
  t.anisotropy = 4;
  return t;
};

export const Moedas: React.FC<{ dados?: Record<string, unknown>; cor: string }> = ({ dados }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const pilhas = ((Array.isArray(dados?.pilhas) ? (dados!.pilhas as Pilha[]) : [{ ano: "2024", itens: 8 }, { ano: "2025", itens: 14 }, { ano: "2026", itens: 20 }]) as Pilha[]).slice(0, 5);
  const maxValor = Math.max(1, ...pilhas.map((p) => (typeof p.valor === "number" ? p.valor : 0)));
  const placas = useMemo(() => pilhas.map((p, i) => placa(String(p.ano ?? ""), String(p.rotulo ?? ""), corDe(p.cor, i))), [JSON.stringify(pilhas)]);
  const n = pilhas.length;
  const entradaMesa = spring({ frame, fps, config: { damping: 18 } });
  const RAIO = 0.5, ESPESSURA = 0.1, TOPO = -1.5;
  return (
    <group position={[0, 0.2, 0]} rotation={[0.55, Math.sin(frame * 0.012) * 0.25, 0]} scale={0.6 * entradaMesa}>
      {/* mesa redonda bege */}
      <mesh position={[0, TOPO - 0.2, 0]}>
        <cylinderGeometry args={[4.4, 4.4, 0.4, 72]} />
        <meshStandardMaterial color="#d8c6a0" roughness={0.55} metalness={0.05} />
      </mesh>
      <mesh position={[0, TOPO - 0.42, 0]}>
        <cylinderGeometry args={[4.5, 4.5, 0.06, 72]} />
        <meshStandardMaterial color="#8a7a58" roughness={0.7} />
      </mesh>
      {pilhas.map((p, i) => {
        const x = n === 1 ? 0 : -2.6 + (5.2 * i) / (n - 1);
        const z = n === 1 ? 0 : (i % 2 ? 0.6 : -0.3);
        const moedas = Math.max(1, Math.min(40, Math.round(typeof p.itens === "number" ? p.itens : typeof p.valor === "number" ? Math.max(3, (p.valor / maxValor) * 26) : 10)));
        const cor = corDe(p.cor, i);
        const inicio = 10 + i * 14;
        const alturaTotal = moedas * ESPESSURA;
        const pop = spring({ frame: frame - inicio - moedas * 1.2, fps, config: { damping: 12 } });
        return (
          <group key={i} position={[x, TOPO, z]}>
            {Array.from({ length: moedas }, (_, k) => {
              const queda = spring({ frame: frame - inicio - k * 1.2, fps, config: { damping: 13 } });
              const y = RAIO * 0 + k * ESPESSURA + ESPESSURA / 2 + (1 - queda) * 5;
              return (
                <mesh key={k} position={[Math.sin(k * 2.1) * 0.02, y, Math.cos(k * 1.7) * 0.02]} rotation={[0, k * 0.4, 0]} scale={[1, 1, 1]}>
                  <cylinderGeometry args={[RAIO, RAIO, ESPESSURA * 0.92, 40]} />
                  <meshStandardMaterial color={cor} emissive={cor} emissiveIntensity={0.28} roughness={0.35} metalness={0.4} opacity={Math.min(1, queda * 3)} transparent />
                </mesh>
              );
            })}
            <mesh position={[0, alturaTotal + 0.62 + (1 - pop) * 1.2, 1.05]} scale={pop}>
              <planeGeometry args={[1.9, 0.82]} />
              <meshBasicMaterial map={placas[i]} transparent />
            </mesh>
          </group>
        );
      })}
    </group>
  );
};
