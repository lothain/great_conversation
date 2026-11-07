"""Minimal local agent: feeds SOURCING.md to the Qwen model on Ollama and runs the shell
commands it requests. Run from the project root (where SOURCING.md, tools/, data/ live).

    python3 agent.py          # approve each command
    python3 agent.py --yes    # run commands without asking
    python3 agent.py --host http://plex-hq-7f72-1:11434   # when not running on the GPU node
"""
import json, subprocess, sys, urllib.request

HOST = sys.argv[sys.argv.index("--host") + 1] if "--host" in sys.argv else "http://localhost:11434"
URL = HOST.rstrip("/") + "/api/chat"
MODEL = "qwen3.6:35b"
AUTO = "--yes" in sys.argv

tools = [{"type": "function", "function": {
    "name": "run", "description": "Run a bash command in the project directory and return its output.",
    "parameters": {"type": "object", "properties": {"cmd": {"type": "string"}}, "required": ["cmd"]}}}]

plan = open("SOURCING.md", encoding="utf-8").read()
msgs = [
    {"role": "system", "content": "You are an agent on a headless Linux server with a bash tool. "
     "Carry out the plan step by step. Inspect files before editing them. If a step needs a human "
     "(a browser login, reading a scan by eye), skip it and list it at the end. Finish with a summary."},
    {"role": "user", "content": plan}]


def chat():
    body = json.dumps({"model": MODEL, "messages": msgs, "tools": tools, "stream": False,
                       "keep_alive": "30m", "options": {"num_ctx": 32768}}).encode()
    req = urllib.request.Request(URL, body, {"Content-Type": "application/json"})
    return json.load(urllib.request.urlopen(req, timeout=1800))["message"]


for step in range(100):
    m = chat()
    msgs.append(m)
    if m.get("content"):
        print("\n[model]", m["content"])
    calls = m.get("tool_calls") or []
    if not calls:
        break
    for c in calls:
        args = c["function"]["arguments"]
        if isinstance(args, str):
            args = json.loads(args)
        cmd = args.get("cmd", "")
        print(f"\n$ {cmd}")
        if not AUTO and input("run? [Y/n] ").lower().startswith("n"):
            out = "User declined to run this command."
        else:
            try:
                r = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=900)
                out = (r.stdout + r.stderr)[-8000:] or f"(exit {r.returncode}, no output)"
            except subprocess.TimeoutExpired:
                out = "Command timed out after 900s."
            print(out[:2000])
        msgs.append({"role": "tool", "content": out})
