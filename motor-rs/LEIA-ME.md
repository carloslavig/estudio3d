# Motor do Carlão (motor-rs)

Motor de render em Rust do Solrac Estúdio. Lê um `roteiro.json`, desenha os quadros e chama o `ffmpeg` para gravar o mp4. Não acessa a internet.

Roda em Windows, macOS (Intel e Apple Silicon) e Linux (x86 e ARM), sem mexer no código.

## Precisa ter

- Rust (https://rustup.rs) para compilar.
- `ffmpeg` no PATH.
- Uma pasta do estúdio com `projetos/<nome>/roteiro.json` (e o áudio que o roteiro citar).

## Compilar e rodar

    cargo build --release
    ./target/release/motor-rs <projeto>               # vídeo inteiro em edicoes/<projeto>/video-rs.mp4
    ./target/release/motor-rs <projeto> --quadro=4.5  # um quadro só (PNG)
    ./target/release/motor-rs <projeto> --ate=10      # só os 10 primeiros segundos

Rode de dentro da pasta do estúdio (a que tem `projetos/`) ou da pasta `motor-rs`.
O `motor-rs.exe` do zip é só para Windows; nos outros sistemas compile.

## O que ele escolhe sozinho

- **Placa de vídeo:** usa a que houver. Metal no Mac, Vulkan ou DX12 no Windows e no Linux, OpenGL em placa antiga. Prefere a dedicada à integrada. Sem placa, desenha pelo processador (todos os núcleos). `ESTUDIO_3D=cpu` força o processador.
- **Gravação do vídeo:** testa o encoder da placa (NVIDIA, Apple VideoToolbox, AMD, Intel) e usa o primeiro que funcionar; sem nenhum, usa o `libx264`. `MOTOR_ENCODER=nome` força um; `--cpu` força o `libx264`.
- **Fontes:** procura nas pastas de fontes do sistema. Pede as do Windows (Arial, Arial Black, Courier New, Consolas, Segoe UI) e, se faltar alguma, usa a equivalente: Arial e Courier New no Mac; Liberation, DejaVu ou Noto no Linux. `MOTOR_FONTES=pasta` aponta outra pasta, e uma pasta `fontes` ao lado do programa também vale.

## Diferenças entre sistemas

- Sem Segoe UI e Consolas (Mac e Linux), o texto sai em Arial e Courier New: muda um pouco a largura das letras.
- Sombras só existem no caminho da placa de vídeo.
- Linux sem nenhuma dessas fontes: instale `fonts-liberation` ou `fonts-dejavu`.

## Outras variáveis

`ESTUDIO_SAIDA=720` grava em 720x1280 (teste leve); `ESTUDIO_MB=85` é o tamanho-alvo do arquivo oficial; `MOTOR_SAT`, `MOTOR_LUZ`, `MOTOR_SOMBRA`, `MOTOR_CAM`, `MOTOR_FAIXA` afinam o visual magno.
