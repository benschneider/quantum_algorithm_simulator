#!/bin/bash

# Script to build and deploy the WebAssembly version to GitHub Pages
# This script builds the quantsim_ui crate and moves the output to the docs/ folder

set -e  # Exit on any error

echo "Building WebAssembly release..."

# Build the WASM release
cd crates/quantsim_ui
trunk build --release

echo "Moving built files to docs/..."

# Move the built files to docs/
cd ../..
cp crates/quantsim_ui/dist/*.js docs/
cp crates/quantsim_ui/dist/*.wasm docs/
cp crates/quantsim_ui/dist/index.html docs/

# Check what files were copied
echo "Copied files:"
ls -la docs/*.js docs/*.wasm docs/index.html

echo "Updating index.html..."

# Update index.html title and base href
cd docs
sed -i.bak 's|<base href="[^"]*"|<base href="/quantum_algorithm_simulator/"|g' index.html

# Remove backup files
rm -f index.html.bak

echo "Web deployment complete!"
echo "Files updated in docs/ folder:"
ls -la *.js *.wasm index.html