"""Opt-in real Pi protocol smoke. Temporary HOME/config/cwd only; NO prompts or package installs.
Run: PI_DESKTOP_PI_BIN=/path/to/pi python3 tests/real_harness.py
"""
import json
import os
import queue
import shutil
import subprocess
import tempfile
import threading
import uuid
from pathlib import Path

binary = os.environ.get('PI_DESKTOP_PI_BIN') or shutil.which('pi')
if not binary:
    raise SystemExit('Pi is not installed; set PI_DESKTOP_PI_BIN')
with tempfile.TemporaryDirectory(prefix='pi-console-rpc-test-') as temp:
    root = Path(temp)
    agent = root / 'agent'
    agent.mkdir()
    session = root / 'fixture.jsonl'
    entries = [
        {'type': 'session', 'version': 3, 'id': str(uuid.uuid4()), 'timestamp': '2026-01-01T00:00:00.000Z', 'cwd': temp},
        {'type': 'message', 'id': 'aaaaaaaa', 'parentId': None, 'timestamp': '2026-01-01T00:00:01.000Z', 'message': {'role': 'user', 'content': 'Duplicate fixture prompt', 'timestamp': 1000}},
        {'type': 'message', 'id': 'bbbbbbbb', 'parentId': 'aaaaaaaa', 'timestamp': '2026-01-01T00:00:02.000Z', 'message': {'role': 'user', 'content': 'Duplicate fixture prompt', 'timestamp': 2000}},
    ]
    session.write_text(''.join(json.dumps(e) + '\n' for e in entries))
    extension = root / 'approval.ts'
    extension.write_text('''export default function(pi) {
      pi.on("session_before_fork", async (_event, ctx) => {
        const ok = await ctx.ui.confirm("Fixture fork", "Confirm test-only fork?");
        return { cancel: !ok };
      });
    }''')
    # Do not inherit provider keys or real configuration overrides.
    env = {k: os.environ[k] for k in ['PATH', 'TMPDIR', 'LANG'] if k in os.environ}
    env.update(HOME=temp, PI_CODING_AGENT_DIR=str(agent), PI_CODING_AGENT_SESSION_DIR=str(root / 'sessions'), PI_OFFLINE='1', PI_TELEMETRY='0')
    process = subprocess.Popen([binary, '--mode', 'rpc', '--offline', '--no-approve', '--no-extensions', '-e', str(extension), '--no-skills', '--no-prompt-templates', '--no-themes', '--no-context-files', '--session', str(session)], cwd=temp, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    events = queue.Queue()
    def read():
        for line in process.stdout:
            try:
                events.put(json.loads(line))
            except ValueError:
                pass
    threading.Thread(target=read, daemon=True).start()
    def send(value):
        process.stdin.write((json.dumps(value) + '\n').encode())
        process.stdin.flush()
    def receive(predicate):
        while True:
            event = events.get(timeout=20)
            if predicate(event):
                return event
    def call(type, **fields):
        id = str(uuid.uuid4())
        send(dict(id=id, type=type, **fields))
        response = receive(lambda e: e.get('id') == id and e.get('type') == 'response')
        assert response['success'], response
        return response.get('data', {})
    try:
        state = call('get_state')
        assert state['sessionId'] == entries[0]['id']
        forks = call('get_fork_messages')['messages']
        assert [m['entryId'] for m in forks] == ['aaaaaaaa', 'bbbbbbbb'], forks
        loaded_entries = call('get_entries')
        assert {'aaaaaaaa', 'bbbbbbbb'} <= {e['id'] for e in loaded_entries['entries']}
        assert loaded_entries['leafId'] in {e['id'] for e in loaded_entries['entries']}
        for confirmed in [False, True]:
            send({'type': 'fork', 'entryId': 'bbbbbbbb', 'id': 'fork-request'})
            request = receive(lambda e: e.get('type') == 'extension_ui_request' and e.get('method') == 'confirm')
            # A second command must be serviced while the first waits for approval.
            assert call('get_state')['sessionId'] == state['sessionId']
            send({'type': 'extension_ui_response', 'id': request['id'], 'confirmed': confirmed})
            response = receive(lambda e: e.get('id') == 'fork-request' and e.get('type') == 'response')
            assert response['success'], response
            assert response['data']['cancelled'] == (not confirmed), response
            if confirmed:
                assert response['data']['text'] == 'Duplicate fixture prompt', response
                assert call('get_state')['sessionId'] != state['sessionId']
        call('abort')
        print('PASS: real Pi state, entries, duplicate fork IDs, concurrent approval channel, cancelled/successful fork, abort. No prompts sent.')
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
