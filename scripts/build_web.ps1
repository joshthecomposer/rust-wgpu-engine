param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$distDir = Join-Path $repoRoot "dist"

if ($Profile -eq "release") {
    cargo build --release --target wasm32-unknown-unknown --no-default-features --features web
} else {
    cargo build --target wasm32-unknown-unknown --no-default-features --features web
}
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed"
}

if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) {
    throw "wasm-bindgen CLI was not found. Install it with: cargo install wasm-bindgen-cli"
}

$metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed"
}

$targetDir = Join-Path $metadata.target_directory "wasm32-unknown-unknown\$Profile"
$wasmInput = Join-Path $targetDir "learn-opengl-rs.wasm"
if (-not (Test-Path $wasmInput)) {
    $wasmInput = Get-ChildItem -Path $targetDir -Filter "*.wasm" -File |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $wasmInput) {
    throw "Could not find wasm output in $targetDir"
}

Remove-Item -Recurse -Force $distDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $distDir | Out-Null

wasm-bindgen `
    --target web `
    --out-dir $distDir `
    --out-name learn_opengl_rs `
    $wasmInput
if ($LASTEXITCODE -ne 0) {
    throw "wasm-bindgen failed"
}

Copy-Item (Join-Path $repoRoot "web\index.html") (Join-Path $distDir "index.html")

$assetRoots = @(
    "resources\shaders",
    "resources\models\static",
    "resources\models\animated",
    "resources\textures",
    "resources\fonts",
    "resources\ui"
)

foreach ($root in $assetRoots) {
    $source = Join-Path $repoRoot $root
    if (-not (Test-Path $source)) {
        continue
    }

    $destination = Join-Path $distDir $root
    New-Item -ItemType Directory -Force (Split-Path -Parent $destination) | Out-Null
    Copy-Item $source $destination -Recurse -Force
}

$configFiles = Get-ChildItem -Path (Join-Path $repoRoot "config") -Filter "*.json" -File
# Includes emitter_data.json (ParticleSystem presets) and game_config.json for wasm.
foreach ($file in $configFiles) {
    $relative = "config\$($file.Name)"
    $destination = Join-Path $distDir $relative
    New-Item -ItemType Directory -Force (Split-Path -Parent $destination) | Out-Null
    Copy-Item $file.FullName $destination -Force
}

$assetManifest = Get-ChildItem -Path $distDir -Recurse -File |
    Where-Object { $_.Name -notin @("index.html", "learn_opengl_rs.js", "learn_opengl_rs_bg.wasm", "asset-manifest.json") } |
    ForEach-Object {
        $relative = $_.FullName.Substring($distDir.Length).TrimStart("\", "/")
        $relative.Replace("\", "/")
    } |
    Sort-Object

# UTF-8 without BOM — BOM breaks `fetch(...).json()` in browsers for this manifest.
$manifestPath = Join-Path $distDir "asset-manifest.json"
$json = $assetManifest | ConvertTo-Json
$utf8NoBom = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText($manifestPath, $json, $utf8NoBom)

Write-Host "WASM package ready: $distDir"
