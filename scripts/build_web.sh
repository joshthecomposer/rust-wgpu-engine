#!/usr/bin/env bash
set -e
cd "$(dirname "$0")/.."

cargo build --target wasm32-unknown-unknown --no-default-features --features web,web_audio

rm -rf dist
mkdir -p dist
wasm-bindgen --target web --out-dir dist --out-name learn_opengl_rs \
  target/wasm32-unknown-unknown/debug/learn-opengl-rs.wasm

cp web/index.html dist/
cp web/game_fmod_bridge.js dist/
cp -r resources config dist/

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
    "learn_opengl_rs.js",
    "learn_opengl_rs_bg.wasm",
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
