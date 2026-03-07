#!/bin/bash

# Script to build and deploy the WebAssembly version to GitHub Pages
# This script builds the quantsim_ui crate and syncs the output to the docs/ folder

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
DIST_DIR="$ROOT_DIR/crates/quantsim_ui/dist"
DOCS_DIR="$ROOT_DIR/docs"

echo "Building WebAssembly release..."

# Build the WASM release
cd "$ROOT_DIR/crates/quantsim_ui"
trunk build --release

echo "Syncing built files to docs/..."

# Remove old generated web artifacts that can go stale between releases.
rm -f "$DOCS_DIR"/*.js "$DOCS_DIR"/*.wasm "$DOCS_DIR"/index.html \
      "$DOCS_DIR"/favicon.ico "$DOCS_DIR"/manifest.json "$DOCS_DIR"/sw.js
rm -rf "$DOCS_DIR"/assets

# Copy the fresh build output.
cp -R "$DIST_DIR"/. "$DOCS_DIR"/

# Check what files were copied
echo "Copied files:"
ls -la "$DOCS_DIR"/*.js "$DOCS_DIR"/*.wasm "$DOCS_DIR"/index.html

echo "Updating index.html..."

# Update index.html title, base href, and file paths
cd "$DOCS_DIR"
sed -i.bak 's|<base href="[^"]*"|<base href="/quantum_algorithm_simulator/"|g' index.html
sed -i.bak "s|from '/quantsim_ui.js'|from './quantsim_ui.js'|g" index.html
sed -i.bak "s|module_or_path: '/quantsim_ui_bg.wasm'|module_or_path: './quantsim_ui_bg.wasm'|g" index.html

# Remove backup files
rm -f index.html.bak

echo "Web deployment complete!"
echo "Files updated in docs/ folder:"
ls -la *.js *.wasm index.html
