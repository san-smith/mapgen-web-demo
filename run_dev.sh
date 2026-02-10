#!/bin/bash
set -e

echo "🚀 Building WASM module..."
wasm-pack build --target web --out-dir pkg --out-name mapgen

# Оптимизация WASM (уменьшает размер на 30-50%)
if command -v wasm-opt &> /dev/null; then
    echo "🔧 Optimizing WASM with wasm-opt..."
    wasm-opt -Oz pkg/mapgen_bg.wasm -o pkg/mapgen_bg.wasm
    echo "✅ WASM optimized"
else
    echo "⚠️  wasm-opt not found. Install with: npm install -g binaryen"
fi

echo "📦 Copying frontend assets..."
mkdir -p pkg
cp -r www/* pkg/

echo "✅ Build successful! Starting server..."
cd pkg

if command -v basic-http-server &> /dev/null; then
    echo "📡 Using basic-http-server"
    basic-http-server -a 0.0.0.0:8080
else
    echo "⚠️  Warning: Using python server (may have MIME issues)"
    python3 -m http.server 8080
fi