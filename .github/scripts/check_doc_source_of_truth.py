#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
FILES = [
    ROOT / 'README.md',
    ROOT / 'USAGE.md',
    ROOT / 'PARITY.md',
    ROOT / 'PHILOSOPHY.md',
    ROOT / 'ROADMAP.md',
    ROOT / '.github' / 'FUNDING.yml',
]
FILES.extend(sorted((ROOT / 'docs').rglob('*.md')) if (ROOT / 'docs').exists() else [])

FORBIDDEN = {
    r'github\.com/Yeachan-Heo/claw-code(?!-parity)': 'replace old claw-code GitHub links with ultraworkers/claw-code',
    r'github\.com/code-yeongyu/claw-code': 'replace stale alternate claw-code GitHub links with ultraworkers/claw-code',
    r'discord\.gg/6ztZB9jvWq': 'replace the stale UltraWorkers Discord invite with the current invite',
    r'api\.star-history\.com/svg\?repos=Yeachan-Heo/claw-code': 'update star-history embeds to ultraworkers/claw-code',
    r'star-history\.com/#Yeachan-Heo/claw-code': 'update star-history links to ultraworkers/claw-code',
    r'assets/clawd-hero\.jpeg': 'rename stale hero asset references to assets/claw-hero.jpeg',
    r'assets/instructkr\.png': 'remove stale instructkr image references',
}

errors: list[str] = []
for path in FILES:
    if not path.exists():
        continue
    text = path.read_text(encoding='utf-8')
    for pattern, message in FORBIDDEN.items():
        for match in re.finditer(pattern, text):
            line = text.count('\n', 0, match.start()) + 1
            errors.append(f'{path.relative_to(ROOT)}:{line}: {message}')

if errors:
    print('doc source-of-truth check failed:', file=sys.stderr)
    for error in errors:
        print(f'  - {error}', file=sys.stderr)
    sys.exit(1)

print('doc source-of-truth check passed')
