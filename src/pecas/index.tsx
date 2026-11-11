import React from "react";
import { spring, useCurrentFrame, useVideoConfig } from "remotion";

type Dados = Record<string, unknown>;
const num = (v: unknown, padrao: number) => (typeof v === "number" ? v : padrao);

// BARRAS: grafico de barras 3D que cresce em sequencia. dados.valores = numeros.
export const Barras: React.FC<{ dados?: Dados; cor: string }> = ({ dados, cor }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const valores = Array.isArray(dados?.valores) ? (dados!.valores as number[]) : [3, 5, 2, 6];
  const max = Math.max(...valores, 1);
  const larg = 0.8;
  const folga = 0.3;
  const total = valores.length * (larg + folga) - folga;
  return (
    <group scale={Math.min(1, 4.4 / total)} rotation={[0.3, -0.5 + frame * 0.006, 0]} position={[0, 0.9, 0]}>
      {valores.map((v, i) => {
        const s = spring({ frame: frame - i * 6, fps, config: { damping: 14 } });
        const h = Math.max(0.02, (v / max) * 3.4 * s);
        return (
          <mesh key={i} position={[-total / 2 + larg / 2 + i * (larg + folga), h / 2 - 1.7, 0]}>
            <boxGeometry args={[larg, h, larg]} />
            <meshStandardMaterial color={cor} roughness={0.35} metalness={0.25} />
          </mesh>
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
