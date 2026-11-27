"""Run a command, capturing its output to PREFIX.log and its exit to PREFIX.exit.

Usage: cap.py PREFIX [ENV=VALUE ...] -- COMMAND ...
Prints the exit and the last lines of the log. Exits with the command's exit.
"""
import os, pathlib, subprocess, sys
args = sys.argv[1:]
prefix = pathlib.Path(args.pop(0))
env = dict(os.environ)
while args and args[0] != '--':
    key, value = args.pop(0).split('=', 1)
    env[key] = value
args.pop(0)
prefix.parent.mkdir(parents=True, exist_ok=True)
with open(str(prefix) + '.log', 'wb') as log:
    code = subprocess.run(args, env=env, cwd=env.get('CAP_CWD'), stdout=log, stderr=subprocess.STDOUT).returncode
pathlib.Path(str(prefix) + '.exit').write_text(f'{code}\n')
tail = pathlib.Path(str(prefix) + '.log').read_text(errors='replace').splitlines()[-int(env.get('CAP_TAIL', '15')):]
print(f'exit={code}')
print('\n'.join(tail))
sys.exit(code)
