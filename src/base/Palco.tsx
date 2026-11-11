import React from "react";
import { useVideoConfig } from "remotion";
import { ThreeCanvas } from "@remotion/three";

// Palco 3D transparente: camera e luzes fixas; a peca entra como filho.
export const Palco: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { width, height } = useVideoConfig();
  return (
    <ThreeCanvas width={width} height={height} camera={{ position: [0, 0, 13], fov: 40 }} style={{ position: "absolute", inset: 0 }}>
      <ambientLight intensity={0.8} />
      <directionalLight position={[4, 6, 8]} intensity={1.8} />
      <pointLight position={[-5, -3, 4]} intensity={30} color="#8ab4ff" />
      {children}
    </ThreeCanvas>
  );
};
