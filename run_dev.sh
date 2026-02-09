#!/bin/bash
set -e

echo "🚀 Building WASM module..."
wasm-pack build --target web --out-dir pkg --out-name mapgen

echo "📦 Copying frontend assets..."
mkdir -p pkg
cp -r www/* pkg/

echo "✅ Build successful! Starting server..."
cd pkg

# Используем правильный сервер
if command -v basic-http-server &> /dev/null; then
    basic-http-server -a 0.0.0.0:8080
else
    echo "⚠️  Warning: Using python server (may have MIME issues)"
    echo "💡 Install http-server: npm install -g http-server"
    python3 -m http.server 8080
fi