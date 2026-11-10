#!/bin/sh
# Monta dist/<nome>: os dois binários, o cofre de exemplo, o ícone e as licenças.
# Vale para Linux e Windows. O macOS tem pacote próprio em scripts/bundle.sh.
#
#   scripts/package.sh Omnimind-linux-x86_64
#   scripts/package.sh Omnimind-windows-x86_64 .exe
set -e
cd "$(dirname "$0")/.."
NAME="${1:?nome do pacote}"
EXT="${2:-}"
cargo build --release -p app --bin omnimind-app
cargo build --release -p omnimind-cli --bin omnimind
OUT="dist/$NAME"
rm -rf "$OUT"
mkdir -p "$OUT"
# O app não pode se chamar "Omnimind" ao lado de "omnimind": Windows e macOS não diferenciam caixa.
cp "target/release/omnimind-app$EXT" "$OUT/OmnimindApp$EXT"
cp "target/release/omnimind$EXT" "$OUT/omnimind$EXT"
# Cofre de exemplo, fictício: o botão "Ver um exemplo" copia daqui.
EX="$OUT/example"
if "$OUT/omnimind$EXT" init "$EX" --name Exemplo >/dev/null \
  && "$OUT/omnimind$EXT" import-markdown fixtures/brains/personal --brain "$EX" >/dev/null \
  && "$OUT/omnimind$EXT" import-markdown fixtures/brains/work --brain "$EX" >/dev/null; then
  :
else
  # Sem o exemplo o app ainda funciona: o botão só não acha o que copiar.
  echo "::warning::could not build the example vault for $NAME; the package ships without it"
  rm -rf "$EX"
fi
cp assets/fonts/OFL-*.txt "$OUT/"
cp assets/icon/icon-1024.png "$OUT/icon.png"
cat > "$OUT/README.txt" <<T
Omnimind

Abra OmnimindApp$EXT e clique em Começar. O cofre nasce na pasta Omnimind, dentro da sua pasta pessoal.

Para ligar a sua IA em todos os projetos, rode no terminal, a partir desta pasta:

  ./omnimind$EXT connect claude-code --global --brain <pasta do cofre>

Use --print para ver o que mudaria sem gravar nada.

No Linux o app precisa de Vulkan ou OpenGL, e de libxkbcommon. O seletor de pasta usa o portal
do ambiente gráfico (xdg-desktop-portal).

Este pacote foi gerado pelo GitHub Actions. O macOS é a plataforma de referência do projeto.
Código e guia: https://github.com/JonathanSantos/omnimind
T
du -sh "$OUT"
