"""Generate the offline cache manifest from Trunk's finished build."""

import base64
import hashlib
import json
import os
from pathlib import Path

staging = Path(os.environ["TRUNK_STAGING_DIR"])
worker = staging / "sw.js"
assets = {}
for path in sorted(staging.rglob("*")):
    if path.is_file() and path != worker:
        digest = hashlib.sha256(path.read_bytes()).digest()
        assets[path.relative_to(staging).as_posix()] = "sha256-" + base64.b64encode(digest).decode()

release = hashlib.sha256(json.dumps(assets).encode()).hexdigest()[:20]
if b"{{__TRUNK_ADDRESS__}}" in (staging / "index.html").read_bytes():
    # Trunk serve substitutes its live-reload address on each request.
    assets["index.html"] = ""

template = Path(__file__).with_name("assets") / "sw.js"
worker.write_text(
    f'const release = "{release}";\nconst assets = {json.dumps(assets, indent=2)};\n\n'
    + template.read_text(encoding="utf-8"),
    encoding="utf-8",
)
