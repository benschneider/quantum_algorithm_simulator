#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")" && pwd)"
cd "$repo_dir/crates/quantsim_ui"
env -u NO_COLOR trunk build --release --locked --public-url ./

# Copy the complete build before pruning superseded Trunk files. docs/ also
# contains hand-maintained files such as the README's demo.gif.
docs_dir="$repo_dir/docs"
mkdir -p "$docs_dir"
cp -R dist/. "$docs_dir/"
for asset in "$docs_dir"/quantsim_ui*.js "$docs_dir"/quantsim_ui*.wasm "$docs_dir"/favicon*.ico; do
    if [[ -f "$asset" && ! -f "dist/${asset##*/}" ]]; then
        rm -- "$asset"
    fi
done
