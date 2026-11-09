#!/usr/bin/env python3
"""POSIX CLI contract test with a fake Godot; not rendering validation.
Usage: python3 tests/e2e/record_logs.py /absolute/path/to/gdx
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

binary = str(Path(sys.argv[1]).resolve())
with tempfile.TemporaryDirectory(prefix='gdx-record-logs-') as tmp:
    root = Path(tmp)
    (root / 'project.godot').write_text('config_version=5\n')
    fake = root / 'fake-godot'
    fake.write_text('''#!/usr/bin/env python3
import os, pathlib, sys
print('OpenGL test device (mock)')
print('fallback warning (mock)', file=sys.stderr)
mode = os.environ.get('GDX_LOG_TEST_MODE', 'ok')
if mode == 'fail': sys.exit(7)
if mode != 'missing': pathlib.Path(sys.argv[sys.argv.index('--write-movie')+1]).write_bytes(b'mock artifact')
''')
    fake.chmod(0o700)
    common = [binary, '--godot', str(fake), '--project', str(root), 'capture', 'record',
              '--scene', 'res://main.tscn', '--duration', '.01', '--fps', '30']
    def run(name, mode='ok', logs=True):
        env = dict(os.environ, GDX_LOG_TEST_MODE=mode)
        cmd = common + ['--out', str(root / (name + '.avi'))]
        if logs:
            cmd += ['--logs-dir', str(root / 'logs')]
        result = subprocess.run(cmd, capture_output=True, text=True, env=env, timeout=10)
        payload = json.loads((result.stdout if result.returncode == 0 else result.stderr).strip().splitlines()[-1])
        assert (result.returncode == 0) == (mode == 'ok'), payload
        return payload
    first = run('first')
    second = run('second')
    assert first['elapsed_ms'] >= 0
    assert first['artifacts']['stdout_log'] != second['artifacts']['stdout_log']
    for payload in (first, second, run('failed', 'fail'), run('missing', 'missing')):
        artifacts = payload['artifacts'] if payload['ok'] else payload['details']['artifacts']
        assert 'OpenGL test device (mock)' in Path(artifacts['stdout_log']).read_text()
        assert 'fallback warning (mock)' in Path(artifacts['stderr_log']).read_text()
    before = set((root / 'logs').iterdir())
    old = run('default', logs=False)
    assert 'stdout_log' not in old['artifacts']
    assert set((root / 'logs').iterdir()) == before
print('PASS: success, repeat/non-overwrite, nonzero exit, missing movie, default compatibility')
