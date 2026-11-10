#!/usr/bin/env python3
"""Sessões reais de IA, isoladas, com grupo de controle sem memória.

Mede correções repetidas: depois que a pessoa corrige algo, as sessões seguintes acertam sem
ninguém lembrar? Os hooks rodam pelo binário real e o contexto vai no prompt, porque
`--setting-sources ""` desliga hooks de propósito.
"""
import json, os, re, shutil, subprocess, sys, tempfile, time, uuid

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BIN = os.path.join(ROOT, "target/release/omnimind")
BUDGET_TOTAL = float(os.environ.get("BUDGET", "1.9"))
MODEL = "sonnet"
TOOLS = "mcp__omnimind__memory_search,mcp__omnimind__memory_read,mcp__omnimind__memory_write,mcp__omnimind__memory_agenda"
PROJECT = "/tmp/fictional/cuscuz"

# (rótulo, mensagem, grupos que a recebem, checagem)
SESSIONS = [
    ("s1-http", "No serviço Cuscuz, escreva uma função Rust que baixa o conteúdo de uma URL. Só o código, curto.", "AC", ("sync_http",)),
    ("s2-correct-http", "Não, errado. No Cuscuz é tudo síncrono: sem tokio e sem async. Para HTTP usamos o crate ureq. Prefiro assim em todo código do Cuscuz.", "A", ()),
    ("s3-deploy", "Como faço o deploy da Rota Certa?", "AC", ("deploy",)),
    ("s4-correct-deploy", "Na verdade o deploy da Rota Certa é sempre em duas etapas: primeiro a migração do banco, depois a aplicação. Nunca direto. E mensagens de commit aqui seguem o formato tipo(escopo): resumo, em português.", "A", ()),
    ("s5-http2", "Escreva uma função Rust no Cuscuz que consulta dois endpoints e devolve os corpos. Só o código, curto.", "AC", ("sync_http",)),
    ("s6-deploy2", "Vou subir a Rota Certa hoje. Quais os passos, resumidos?", "AC", ("deploy",)),
    ("s7-commit", "Sugira a mensagem de commit para uma mudança que corrige o cálculo de frete na Rota Certa.", "AC", ("commit",)),
    ("s8-all", "Mesmo pedido de sempre: função Rust no Cuscuz para enviar um POST, e depois diga como publicar a Rota Certa. Curto.", "AC", ("sync_http", "deploy")),
]

CHECKS = {
    "sync_http": lambda t: ("ureq" in t.lower()) and not re.search(r"\basync\b|\btokio\b|\.await", strip_negations(t)),
    "deploy": lambda t: bool(re.search(r"migra", t, re.I)) and bool(re.search(r"duas etapas|depois|em seguida|2\.|segund", t, re.I)),
    "commit": lambda t: bool(re.search(r"\b[a-z]+\([a-z0-9_-]+\):", t)),
}


def strip_negations(t):
    # Dizer "sem tokio" não é usar tokio: só o código conta.
    blocks = re.findall(r"```.*?```", t, re.S)
    code = "\n".join(blocks) if blocks else t
    # Comentário "sem tokio" também não é usar tokio.
    return re.sub(r"//.*", "", code)


def hook(brain, event, payload):
    p = subprocess.run([BIN, "hook", event, "--brain", brain], input=json.dumps(payload), capture_output=True, text=True, timeout=10)
    try:
        out = json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return ""
    return (out.get("hookSpecificOutput") or {}).get("additionalContext", "") or out.get("additionalContext", "")


def claude(prompt, mcp_cfg):
    cmd = ["claude", "-p", prompt, "--model", MODEL, "--no-session-persistence", "--setting-sources", "", "--tools", "", "--max-budget-usd", "0.40", "--output-format", "json"]
    if mcp_cfg:
        cmd += ["--strict-mcp-config", "--mcp-config", mcp_cfg, "--allowedTools", TOOLS]
    else:
        cmd += ["--strict-mcp-config", "--mcp-config", '{"mcpServers":{}}']
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=150)
    except subprocess.TimeoutExpired:
        return {"result": "", "cost": 0.0, "error": "timeout"}
    try:
        d = json.loads(p.stdout)
    except json.JSONDecodeError:
        return {"result": "", "cost": 0.0, "error": (p.stdout + p.stderr)[-300:]}
    return {"result": d.get("result", ""), "cost": d.get("total_cost_usd", 0.0), "turns": d.get("num_turns"), "error": d.get("subtype") if d.get("is_error") else None}


def run(arm, brain, cfg, log):
    spent = 0.0
    for label, msg, arms, checks in SESSIONS:
        if arm not in arms:
            continue
        if log["spent"] + 0.35 > BUDGET_TOTAL:
            print("budget stop", file=sys.stderr)
            break
        sid = f"{arm}-{label}-{uuid.uuid4().hex[:6]}"
        base = {"session_id": sid, "cwd": PROJECT, "transcript_path": "", "hook_event_name": ""}
        prompt = msg
        if arm == "A":
            ctx = hook(brain, "session-start", {**base, "hook_event_name": "SessionStart", "source": "startup"})
            pctx = hook(brain, "prompt", {**base, "hook_event_name": "UserPromptSubmit", "prompt": msg})
            parts = [f"<system-context>\n{c}\n</system-context>" for c in (ctx, pctx) if c]
            prompt = "\n".join(parts + [msg])
        r = claude(prompt, cfg if arm == "A" else None)
        if arm == "A":
            hook(brain, "session-end", {**base, "hook_event_name": "SessionEnd", "reason": "exit"})
        log["spent"] += r.get("cost") or 0.0
        verdict = {c: bool(CHECKS[c](r["result"])) for c in checks}
        row = {"arm": arm, "session": label, "cost": round(r.get("cost") or 0, 4), "turns": r.get("turns"), "checks": verdict, "answer": r["result"][:600], "error": r.get("error")}
        log["rows"].append(row)
        json.dump(log, open(log["out"], "w"), ensure_ascii=False, indent=1)
        print(json.dumps({k: row[k] for k in ("arm", "session", "cost", "checks", "error")}, ensure_ascii=False), flush=True)
    return spent


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "qa/ai-sessions/results.json")
    tmp = tempfile.mkdtemp(prefix="omnimind-ai-")
    brain = os.path.join(tmp, "brain")
    subprocess.run([BIN, "init", brain, "--name", "Teste"], check=True, capture_output=True)
    cfg = os.path.join(tmp, "mcp.json")
    json.dump({"mcpServers": {"omnimind": {"command": BIN, "args": ["mcp", "--brain", brain, "--client", "claude-code", "--cwd", PROJECT]}}}, open(cfg, "w"))
    log = {"spent": 0.0, "rows": [], "date": time.strftime("%Y-%m-%d %H:%M"), "out": out}
    arm = sys.argv[2] if len(sys.argv) > 2 else "AC"
    if "A" in arm:
        run("A", brain, cfg, log)
    if "C" in arm:
        run("C", brain, None, log)
    memories = subprocess.run([BIN, "log", "--brain", brain, "--limit", "60"], capture_output=True, text=True).stdout
    log["journal_tail"] = memories[-3000:]
    portrait = subprocess.run([BIN, "portrait", "--brain", brain], capture_output=True, text=True).stdout
    log["portrait"] = portrait[:2000]
    json.dump(log, open(out, "w"), ensure_ascii=False, indent=1)
    print("spent", round(log["spent"], 3))
    shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
