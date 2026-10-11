#!/usr/bin/env python3
"""Reproducible quoting inventory; comments-with-code count as code lines."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'docs/audit-manifest.json'


def inventory(pattern):
    files = []
    for path in sorted(ROOT.glob(pattern)):
        if not path.is_file():
            continue
        data = path.read_bytes()
        lines = data.decode().splitlines()
        # Current scope uses // comments, not block comments. Fail rather than
        # silently miscount if a later revision introduces a block comment.
        if path.suffix == '.rs' and '/*' in data.decode():
            raise ValueError(f'Block comments require updating the counter: {path}')
        code = sum(bool(line.strip()) and not line.lstrip().startswith('//') for line in lines)
        files.append(dict(path=str(path.relative_to(ROOT)), physicalLines=len(lines),
                          nonblankNonCommentOnlyLines=code,
                          sha256=hashlib.sha256(data).hexdigest()))
    return dict(files=files, physicalLines=sum(f['physicalLines'] for f in files),
                nonblankNonCommentOnlyLines=sum(f['nonblankNonCommentOnlyLines'] for f in files))


def manifest():
    production = inventory('programs/solana_guard/src/**/*.rs')
    production_only = 0
    for file in production['files']:
        source = (ROOT / file['path']).read_text().split('#[cfg(test)]', 1)[0]
        production_only += sum(bool(line.strip()) and not line.lstrip().startswith('//')
                               for line in source.splitlines())
    production['nonblankNonCommentOnlyLinesExcludingEmbeddedTests'] = production_only
    fingerprint = hashlib.sha256(json.dumps(production, sort_keys=True).encode()).hexdigest()
    return dict(version='0.2.0', program='solana_guard',
                productionRust=production,
                integrationTests=inventory('programs/solana_guard/tests/**/*.rs'),
                abiExamples=inventory('programs/solana_guard/examples/**/*.rs'),
                scopeFingerprint=fingerprint,
                method='Physical lines and nonblank lines excluding //-only comments; inline comments on code count. Separate production-only count excludes trailing #[cfg(test)] modules; integration tests/examples listed separately. Dependencies/generated IDL excluded.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    content = json.dumps(manifest(), indent=2) + '\n'
    if args.check:
        if not OUT.exists() or OUT.read_text() != content:
            raise SystemExit('Audit manifest is stale; run python3 scripts/audit-scope.py')
        print('Audit manifest matches scoped source.')
    else:
        OUT.write_text(content)
        print(content)
