# Estúdio 3D (do Carlos): vídeos curtos explicativos em 3D

Projeto próprio e independente, feito do zero com Remotion + three.js. Vertical 1080×1920, 30 fps (Reels, TikTok, Shorts). Nada aqui vem de produto de terceiros.
Quem trabalha na pasta: Claude Code (ou o Jarvis, pela skill `estudio-3d`). O Carlos aprova cada etapa e publica.

## Fluxo de um vídeo (um projeto por vídeo em `projetos/NNN-nome/`)
1. **Tema e roteiro**: pesquisar com fontes, escrever `narracao.txt` (texto falado, frases curtas, ~25 a 45 s) e anotar as fontes em `fontes.md`. Gancho nos 3 primeiros segundos. Não inventar número: sem fonte, não entra.
2. **Narração**: `python tools/narrar.py NNN-nome` gera `public/audio/NNN-nome.wav` (voz do Braniac). Se o Carlos preferir outra voz, ele coloca o `.wav` no mesmo caminho.
3. **Sincronia**: `python tools/transcrever.py NNN-nome` transcreve (Whisper local) e grava `audio` e `legenda` com tempos reais no `roteiro.json`.
4. **Cenas**: ajustar `cenas` do `roteiro.json` aos tempos da legenda (uma ideia por cena, 2 a 5 s). Peças: `barras` (comparar/crescer; `dados.valores`), `globo` (mundo/rede/dados), `pilha` (acumular; `dados.itens`).
5. **Conferir**: `npm run quadro -- NNN-nome --segundo=4` gera um PNG do instante e se olha antes de renderizar. Texto na área segura, nada cortado, legenda legível.
6. **Renderizar**: `npm run render -- NNN-nome` → `edicoes/NNN-nome/video.mp4`.
7. Entregar ao Carlos: vídeo, uma legenda de post (`legenda do post.txt`) e as fontes. Publicar é com ele.

## Pastas
`src/base` (palco 3D, legenda) · `src/pecas` (peças 3D; para uma nova, crie o componente, registre em `PECAS` e no tipo `Cena.peca` em `src/tipos.ts`) · `src/estilos.ts` (cores por perfil) · `src/Estudio.tsx` (monta o vídeo a partir do roteiro) · `projetos/` · `edicoes/` · `tools/` · `scripts/`.

## Regras
- Área segura: nada importante nos ~260 px de baixo nem nos ~120 px das laterais (a interface das redes cobre). A legenda já respeita isso.
- Fontes do sistema (sem licença externa). Música/efeitos só de fonte livre e citada.
- Remotion é gratuito para pessoa física e empresas de até 3 pessoas; acima disso exige licença deles (remotion.dev/license).
- Conteúdo político, saúde e finanças: só com fonte primária citada e linguagem sem promessa. Sem dado pessoal de ninguém nos vídeos.
- Não publica, não posta e não paga nada: entrega o arquivo ao Carlos.
- Requisitos: Node 20+, ffmpeg, e (para narrar/transcrever) o Braniac em `C:\Users\carlo\Projetos\Agente` (variável `BRANIAC_RAIZ` muda o caminho). Instalar: `npm install`.
