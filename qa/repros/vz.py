"""QA helper: drives the real omnimind binary over MCP stdio JSON-RPC and the CLI."""
import json
import os
import pathlib
import subprocess
import uuid

ROOT = pathlib.Path(__file__).resolve().parents[2]
# Pasta de trabalho do QA, fora do repositório. `OMNIMIND_QA` escolhe outra.
QA = pathlib.Path(os.environ.get("OMNIMIND_QA", "/tmp/omnimind-qa"))


# Frozen copy of target/release/omnimind; another session was rebuilding target/ during QA.
VZ = os.environ.get("VZ", str(QA / "bin/omnimind"))


def env(home=None):
    e = dict(os.environ)
    e.pop("OMNIMIND_BRAIN", None)
    e["HOME"] = str(home or (QA / "home"))
    return e


def cli(*args, brain=None, stdin=None, check=False, home=None):
    cmd = [VZ, *map(str, args)]
    if brain:
        cmd += ["--brain", str(brain)]
    p = subprocess.run(cmd, capture_output=True, text=True, input=stdin, env=env(home), timeout=120)
    if check and p.returncode:
        raise RuntimeError(f"{cmd}: rc={p.returncode} {p.stderr}")
    return p


def fresh(name):
    b = QA / name
    if b.exists():
        subprocess.run(["rm", "-rf", str(b)], check=True)
    r = cli("init", b, "--name", name)
    if r.returncode:
        raise RuntimeError(r.stderr + r.stdout)
    return b


class Mcp:
    """One long-lived `omnimind mcp` process."""

    def __init__(self, brain, client="qa-agent", session=None, extra=()):
        self.session = session or ("qa-" + uuid.uuid4().hex[:8])
        self.brain = brain
        cmd = [VZ, "mcp", "--brain", str(brain), "--client", client, "--session", self.session, *extra]
        self.p = subprocess.Popen(
            cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, env=env(), bufsize=1,
        )
        self.n = 0

    def rpc(self, method, params=None):
        self.n += 1
        msg = {"jsonrpc": "2.0", "id": self.n, "method": method}
        if params is not None:
            msg["params"] = params
        self.p.stdin.write(json.dumps(msg, ensure_ascii=False) + "\n")
        self.p.stdin.flush()
        line = self.p.stdout.readline()
        if not line:
            raise RuntimeError("EOF from mcp; stderr=" + self.p.stderr.read())
        return json.loads(line)

    def call(self, tool, **args):
        r = self.rpc("tools/call", {"name": tool, "arguments": args})
        if "error" in r:
            return {"_rpc_error": r["error"]}
        res = r["result"]
        out = res.get("structuredContent")
        if out is None:
            try:
                out = json.loads(res["content"][0]["text"])
            except Exception:
                out = {"_text": res["content"][0]["text"]}
        if res.get("isError"):
            out = dict(out) if isinstance(out, dict) else {"_v": out}
            out["_isError"] = True
        return out

    def write(self, **a):
        return self.call("memory_write", **a)

    def search(self, *q, **a):
        return self.call("memory_search", query=list(q), **a)

    def read(self, id, **a):
        return self.call("memory_read", id=id, **a)

    def close(self):
        try:
            self.p.stdin.close()
            self.p.wait(timeout=10)
        except Exception:
            self.p.kill()
        err = self.p.stderr.read()
        return self.p.returncode, err


def hook(event, brain, payload=None, client=None):
    args = ["hook", event, "--brain", str(brain)]
    if client:
        args += ["--client", client]
    return subprocess.run(
        [VZ, *args], input=json.dumps(payload or {}), capture_output=True, text=True, env=env(), timeout=30
    )


def briefing(brain, cwd=None, session="qa-bf"):
    r = hook("session-start", brain, {"session_id": session, "cwd": str(cwd or brain), "source": "startup"})
    try:
        d = json.loads(r.stdout)
        return d.get("hookSpecificOutput", d).get("additionalContext", r.stdout)
    except Exception:
        return r.stdout
