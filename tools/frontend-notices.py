#!/usr/bin/env python3
"""Collect bundled frontend dependency and font notices without network access."""
from pathlib import Path
import json
root = Path(__file__).resolve().parents[1]
app = root / 'apps/fontes-web'
notices = ['Fontes frontend third-party notices\n']
for manifest in sorted((app / 'node_modules').glob('**/package.json')):
    try:
        package = json.loads(manifest.read_text())
    except (ValueError, OSError):
        continue
    texts = [p for p in manifest.parent.iterdir() if p.is_file() and p.name.lower() in ('license', 'license.md', 'license.txt', 'copying', 'ofl.txt')]
    for path in texts:
        notices.append(f"\n{'='*72}\n{package.get('name', manifest.parent.name)} {package.get('version', '')}\n{path.read_text(errors='replace')}\n")
for path in sorted((root / 'assets/ui/licenses').glob('*.txt')):
    notices.append(f"\n{'='*72}\n{path.name}\n{path.read_text()}\n")
output = app / 'public/THIRD_PARTY_NOTICES.txt'
output.parent.mkdir(exist_ok=True)
output.write_text('\n'.join(line.rstrip() for line in '\n'.join(notices).splitlines()).rstrip() + '\n')
