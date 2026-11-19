// Uso:  npm run render -- 001-exemplo            -> edicoes/001-exemplo/video.mp4
//       npm run render -- 001-exemplo --quadro   -> edicoes/001-exemplo/quadro-<s>.png (confere visual sem renderizar tudo)
//       opcional: --segundo=4.5 (qual instante do quadro)
import { spawnSync } from "node:child_process";
import { mkdirSync, existsSync } from "node:fs";
import { resolve } from "node:path";

const args = process.argv.slice(2);
const projeto = args.find((a) => !a.startsWith("--"));
if (!projeto) {
  console.error("Diga o projeto: npm run render -- <pasta em projetos/>  (ex.: 001-exemplo)");
  process.exit(1);
}
const roteiro = resolve("projetos", projeto, "roteiro.json");
if (!existsSync(roteiro)) {
  console.error(`Nao achei ${roteiro}`);
  process.exit(1);
}
const saida = resolve("edicoes", projeto);
mkdirSync(saida, { recursive: true });

const quadro = args.includes("--quadro");
const seg = Number((args.find((a) => a.startsWith("--segundo=")) ?? "--segundo=2").split("=")[1]);
const npx = process.platform === "win32" ? "npx.cmd" : "npx";
// angle = WebGL pela placa de video: ~30% mais rapido que o padrao (medido 13/11: 139 s -> 95 s num video de 38 s). ESTUDIO_GL=swangle volta ao software.
const comum = ["--props=" + roteiro, "--gl=" + (process.env.ESTUDIO_GL || "angle")];
const cmd = quadro
  ? ["remotion", "still", "src/index.ts", "Estudio", resolve(saida, `quadro-${seg}.png`), `--frame=${Math.round(seg * 30)}`, ...comum]
  : ["remotion", "render", "src/index.ts", "Estudio", resolve(saida, "video.mp4"), "--codec=h264", "--crf=18", ...comum];
const r = spawnSync(npx, cmd, { stdio: "inherit", shell: process.platform === "win32" });
process.exit(r.status ?? 1);
