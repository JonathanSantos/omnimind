#!/usr/bin/env python3
"""Hooks de verdade: o Claude Code carrega o `.claude/settings.json` que `omnimind connect` escreveu.

Tudo acontece numa pasta temporária. `--setting-sources project` lê só o projeto de teste, e
`--strict-mcp-config` usa só o `.mcp.json` gerado. A memória nativa fica desligada e nada persiste.

  real-hooks.py BINÁRIO_OMNIMIND PASTA_TEMPORÁRIA
"""
import json, os, subprocess, sys, time

TOOLS = "mcp__omnimind__memory_search,mcp__omnimind__memory_read,mcp__omnimind__memory_write,mcp__omnimind__memory_agenda"


def claude(project, prompt, connected):
    cmd = ["claude", "-p", prompt, "--model", "sonnet", "--setting-sources", "project", "--no-session-persistence",
           "--tools", "", "--max-budget-usd", "0.40", "--output-format", "stream-json", "--verbose",
           "--include-hook-events", "--strict-mcp-config"]
    if connected:
        cmd += ["--mcp-config", os.path.join(project, ".mcp.json"), "--allowedTools", TOOLS]
    else:
        cmd += ["--mcp-config", '{"mcpServers":{}}']
    env = {**os.environ, "CLAUDE_CODE_DISABLE_AUTO_MEMORY": "1"}
    t0 = time.time()
    run = subprocess.run(cmd, cwd=project, capture_output=True, text=True, timeout=170, env=env)
    out = {"reply": "", "tools": [], "hooks": {}, "mcp": None, "cost": 0.0, "seconds": round(time.time() - t0, 1), "models": [], "contexts": []}
    for line in run.stdout.splitlines():
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        kind, sub = ev.get("type"), ev.get("subtype", "")
        if kind == "system" and sub == "init":
            out["mcp"] = ev.get("mcp_servers")
        elif kind == "system" and "hook" in sub:
            name = ev.get("hook_event") or ev.get("hook_name") or sub
            key = f"{name}:{sub}"
            out["hooks"][key] = out["hooks"].get(key, 0) + 1
            text = ev.get("output") or ev.get("stdout") or ""
            if sub == "hook_response" and text:
                out["contexts"].append(text[:700])
            if ev.get("exit_code") not in (None, 0) or ev.get("outcome") == "error":
                out.setdefault("errors", []).append(str(ev)[:300])
        elif kind == "assistant":
            for b in ev["message"].get("content", []):
                if b.get("type") == "text":
                    out["reply"] += b["text"]
                elif b.get("type") == "tool_use":
                    out["tools"].append(b["name"].split("__")[-1])
        elif kind == "result":
            out["cost"] = ev.get("total_cost_usd") or 0.0
            out["models"] = list((ev.get("modelUsage") or {}).keys())
    return out


def main():
    binary, base = sys.argv[1], sys.argv[2]
    project, brain = os.path.join(base, "projeto"), os.path.join(base, "cofre")
    os.makedirs(project, exist_ok=True)
    home = os.path.join(base, "home")
    os.makedirs(home, exist_ok=True)
    cli = lambda *a: subprocess.run([binary, *a], capture_output=True, text=True, env={**os.environ, "HOME": home}).stdout
    print(cli("init", brain, "--name", "Teste real").splitlines()[0][:60])
    print(cli("connect", "claude-code", "--brain", brain, "--project", project).splitlines()[-1])
    rows = []

    def step(label, prompt, connected=True):
        r = claude(project, prompt, connected)
        r["label"], r["user"] = label, prompt
        rows.append(r)
        print(f"\n### {label} | {r['seconds']}s US$ {r['cost']:.4f} | tools {r['tools']} | mcp {r['mcp']}")
        print("hooks:", r["hooks"], "erros:", r.get("errors"))
        print("IA:", r["reply"][:500])
        log = cli("log", "--brain", brain, "--limit", "200").splitlines()
        print("eventos no diário:", len(log))
        return len(log)

    step("1 ensina", "oi, me chamo Bia e cuido da API da Rota Certa. prefiro respostas curtas, no máximo três linhas. guarda isso.")
    step("2 sessão nova lembra", "como eu me chamo, o que eu faço e como gosto das respostas?")
    step("3 corrige", "não, na verdade prefiro respostas com no máximo cinco linhas.")
    step("4 sessão nova vê a correção", "qual o limite de linhas que eu quero? responda só o número.")
    before = len(cli("log", "--brain", brain, "--limit", "500").splitlines())
    print("\n", cli("disconnect", "claude-code", "--project", project).splitlines()[-1])
    print("arquivos no projeto:", sorted(os.listdir(project)))
    after = step("5 desligado esquece", "como eu me chamo?", connected=False)
    print("diário cresceu com o Omnimind desligado?", after != before, before, after)
    print(cli("connect", "claude-code", "--brain", brain, "--project", project).splitlines()[-1])
    step("6 religado lembra", "como eu me chamo?")
    print("\n--- diário")
    print("\n".join(l[28:110] for l in cli("log", "--brain", brain, "--limit", "60").splitlines()))
    print("--- doctor")
    print(cli("doctor", "--brain", brain))
    print(cli("agenda", "--brain", brain)[:300])
    for f in sorted(os.listdir(os.path.join(brain, "memories", "preference"))):
        print("pref:", f)
    json.dump({"date": "2026-11-01", "rows": rows, "cost_total": round(sum(r["cost"] for r in rows), 4)},
              open(os.path.join(base, "runs.json"), "w"), ensure_ascii=False, indent=1)
    print("total US$", round(sum(r["cost"] for r in rows), 4))


main()
