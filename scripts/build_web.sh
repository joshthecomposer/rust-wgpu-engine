#!/usr/bin/env bash
set -e
cd "$(dirname "$0")/.."

PROFILE="${1:-debug}"
if [ "$PROFILE" = "release" ]; then
  cargo build --release --target wasm32-unknown-unknown --no-default-features --features web,web_audio
  WASM_DIR="release"
else
  cargo build --target wasm32-unknown-unknown --no-default-features --features web,web_audio
  WASM_DIR="debug"
fi

rm -rf dist
mkdir -p dist
wasm-bindgen --target web --out-dir dist --out-name learn_opengl_rs \
  "target/wasm32-unknown-unknown/${WASM_DIR}/learn-wgpu-rs.wasm"

cp web/index.html dist/
cp web/game_fmod_bridge.js dist/

# Match build_web.ps1: only publish asset subtrees needed at runtime (not all of resources/).
ASSET_ROOTS=(
  resources/shaders
  resources/models/static
  resources/models/animated
  resources/textures
  resources/fonts
  resources/ui
)
for root in "${ASSET_ROOTS[@]}"; do
  if [ -d "$root" ]; then
    mkdir -p "dist/$root"
    cp -r "$root/." "dist/$root/"
  fi
done

mkdir -p dist/config
shopt -s nullglob
for f in config/*.json; do
  cp "$f" dist/config/
done
shopt -u nullglob

mkdir -p dist/resources/fmod/Web
if [ -d resources/fmod/Web ]; then
  cp -r resources/fmod/Web/. dist/resources/fmod/Web/
elif [ -d resources/fmod/Desktop ]; then
  cp resources/fmod/Desktop/*.bank dist/resources/fmod/Web/ 2>/dev/null || true
fi

if [ -d third_party/fmod ]; then
  mkdir -p dist/third_party
  cp -r third_party/fmod dist/third_party/
fi

python3 <<'PY'
import os, json

d = "dist"
skip_names = {
    "index.html",
    "learn_wgpu_rs.js",
    "learn_wgpu_rs_bg.wasm",
    "asset-manifest.json",
    "game_fmod_bridge.js",
}


def skip_path(rel):
    rel = rel.replace(chr(92), "/")
    if rel.startswith("resources/fmod/Web/") or rel.startswith("third_party/fmod/"):
        return True
    return False


paths = []
for r, _, fs in os.walk(d):
    for f in fs:
        if f in skip_names:
            continue
        full = os.path.join(r, f)
        rel = os.path.relpath(full, d).replace(chr(92), "/")
        if skip_path(rel):
            continue
        paths.append(rel)
paths.sort()
with open(os.path.join(d, "asset-manifest.json"), "w", encoding="utf-8") as out:
    out.write(json.dumps(paths, indent=2) + "\n")
PY

if [ ! -f dist/resources/fmod/Web/Master.bank ]; then
  echo "WARNING: dist/resources/fmod/Web/Master.bank missing — dist.zip will have no FMOD banks. Add resources/fmod/Desktop or resources/fmod/Web banks before build." >&2
else
  echo "FMOD banks in dist: $(wc -c < dist/resources/fmod/Web/Master.bank) bytes Master.bank"
fi

echo "WASM package ready: $(pwd)/dist"
