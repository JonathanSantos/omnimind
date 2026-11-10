#!/usr/bin/env python3
"""Quanto a memória ajuda quem alterna entre muitos projetos de estudo.

Perfil, projetos e fatos são fictícios. Só o padrão de uso imita um perfil real: muitas stacks,
convenções que se repetem e lições que nascem num projeto e fazem falta em outro.

Dois grupos com o mesmo pedido, em processos novos e sem histórico:
  omnimind  contexto do projeto + MCP + hooks pelo binário real
  control   só o contexto do projeto, que é o que a IA leria no repositório

  multiproject.py seed DIR         cria o cofre sem custo de IA
  multiproject.py run DIR [ids]    roda os casos, um por vez
  multiproject.py report DIR       mostra a comparação
"""
import json, os, random, re, subprocess, sys, time, unicodedata, uuid

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BIN = os.path.join(ROOT, "target/release/omnimind")
TOOLS = "mcp__omnimind__memory_search,mcp__omnimind__memory_read,mcp__omnimind__memory_write,mcp__omnimind__memory_agenda"
CAP = float(os.environ.get("BUDGET", "0.75"))

# Regras gerais: valem em qualquer projeto.
GENERAL = [
    ("Functional core, imperative shell", "Prefere functional core, imperative shell: lógica pura no núcleo e IO só na borda.",
     "prefiro functional core, imperative shell em todo projeto: lógica pura no núcleo e IO só na borda"),
    ("Rust sem unwrap fora de teste", "Em Rust nunca usa unwrap fora de teste: thiserror nas libs e anyhow nos binários.",
     "prefiro em Rust nunca usar unwrap fora de teste: thiserror nas libs e anyhow nos binários"),
    ("Código em inglês, comentários em português", "Código e identificadores em inglês; comentários e documentação em português.",
     "sempre código e identificadores em inglês, comentários e documentação em português"),
    ("Teste nunca escreve no HOME real", "Teste nunca escreve no HOME real, sempre pasta temporária com HOME apontando para ela.",
     "sempre teste nunca escreve no HOME real, sempre pasta temporária com HOME apontando para ela"),
    ("Early return e funções pequenas", "Prefere early return e funções pequenas; trait só quando existem duas implementações reais.",
     "prefiro early return e funções pequenas, trait só quando existem duas implementações reais"),
    ("Decisões em docs/adr", "Gosta de decisões em docs/adr, uma por arquivo.",
     "gosto de decisões em docs/adr, uma por arquivo"),
]

# Lições de cada projeto: (tipo, título, resumo, frase da pessoa).
PROJECTS = {
    "trilho": [
        ("procedure", "Build universal do Tauri 2 no macOS", "O build universal do Tauri 2 quebrava por faltar o target x86_64-apple-darwin; resolvido com rustup target add e tauri build --target universal-apple-darwin.",
         "anota: o build universal do Tauri 2 quebrava por faltar o target x86_64-apple-darwin, resolvi com rustup target add e tauri build --target universal-apple-darwin"),
        ("decision", "Binário de hook do trilho sem tokio", "O binário de hook do trilho fica sem tokio e sem rede, para abrir em menos de 20 ms.",
         "anota: o binário de hook do trilho fica sem tokio e sem rede, para abrir em menos de 20 ms"),
        ("episode", "Trilho parado na tarefa 14", "Parou no trilho na tarefa 14, a tela de diff de permissões; falta o teste de fumaça no Windows.",
         "anota: parei no trilho na tarefa 14, a tela de diff de permissões, falta o teste de fumaça no Windows"),
    ],
    "peneira": [
        ("procedure", "Compilação cruzada com pg_query_go", "O pg_query_go usa CGO e quebrou a compilação cruzada; resolvido com zig cc como CC no CI.",
         "anota: o pg_query_go usa CGO e quebrou a compilação cruzada, resolvi com zig cc como CC no CI"),
        ("decision", "Peneira nega por padrão", "A política da peneira é deny por padrão e fail closed: nó de AST desconhecido recusa.",
         "anota: a política da peneira é deny por padrão e fail closed, nó de AST desconhecido recusa"),
    ],
    "vitrola": [
        ("procedure", "esbuild em extensão VS Code", "Na extensão VS Code, o esbuild precisa marcar vscode como external, senão o bundle quebra na ativação.",
         "anota: na extensão VS Code o esbuild precisa marcar vscode como external, senão o bundle quebra na ativação"),
        ("decision", "Manifesto derivado do TypeScript", "O manifesto da vitrola é derivado do TypeScript, nunca escrito à mão.",
         "anota: o manifesto da vitrola é derivado do TypeScript, nunca escrito à mão"),
    ],
    "ladrilho": [
        ("decision", "Classes CSS fixas no ladrilho", "As classes CSS fixas do ladrilho são API pública, sem hash.",
         "anota: as classes CSS fixas do ladrilho são API pública, sem hash"),
    ],
    "mina-infinita": [
        ("decision", "Seed por chunk na mina-infinita", "O gerador usa seed por chunk com xoshiro256 para ser determinístico.",
         "anota: o gerador da mina-infinita usa seed por chunk com xoshiro256 para ser determinístico"),
        ("episode", "Mina-infinita parada no solver", "Parou na mina-infinita no solver que garante jogada segura sem 50/50; faltam testes de propriedade com proptest e o HUD.",
         "anota: parei na mina-infinita no solver que garante jogada segura sem 50/50, faltam testes de propriedade com proptest e o HUD"),
    ],
    "carta-rapida": [
        ("decision", "Carta-rápida sem build", "A carta-rapida é JavaScript puro com ES modules, sem build, porque é estudo antes de portar para Godot.",
         "anota: a carta-rapida é JavaScript puro com ES modules, sem build, porque é estudo antes de portar para Godot"),
        ("decision", "Core da carta-rápida sem DOM", "O core da carta-rapida é lógica pura sem DOM, para portar depois.",
         "anota: o core da carta-rapida é lógica pura sem DOM, para portar depois"),
    ],
    "estudo-go": [
        ("concept", "Vazamento de goroutine", "Goroutine vaza quando ninguém lê do canal; passar context e sair com select no ctx.Done.",
         "anota: aprendi que goroutine vaza quando ninguém lê do canal, passar context e sair com select no ctx.Done"),
        ("concept", "errors.Is e wrap", "errors.Is percorre a cadeia de wrap; == não enxerga o erro embrulhado com %w.",
         "anota: aprendi que errors.Is percorre a cadeia de wrap, == não enxerga o erro embrulhado com %w"),
        ("concept", "append e array de base", "append pode compartilhar o array de base; usar slices.Clone quando o slice sai da função.",
         "anota: aprendi que append pode compartilhar o array de base, usar slices.Clone quando o slice sai da função"),
    ],
}

# O que a IA leria no repositório. Fica igual nos dois grupos.
REPO = {
    "trilho": "App desktop em Rust com Tauri 2 e React, que inspeciona configuração de agentes.",
    "peneira": "CLI em Go que aplica política de acesso a um Postgres de testes.",
    "vitrola": "Framework em TypeScript para extensões do VS Code.",
    "ladrilho": "Design system em web components com Stencil.",
    "mina-infinita": "Campo minado infinito em Rust para desktop.",
    "carta-rapida": "Protótipo de auto-battler de cartas em JavaScript, com ES modules.",
    "estudo-go": "Exercícios soltos para aprender Go.",
}

FILLER = ["ameixa", "bussola", "caju", "dendê", "farol", "goiaba", "ipe", "jangada", "kombi", "lampiao", "mandacaru", "nautilo",
          "oficina", "pandeiro", "quintal", "rabeca", "sertao", "tamarindo", "umbu", "varanda", "xilogravura", "zabumba", "alpendre", "boneca"]
STACKS = ["Rust com wgpu", "Go com sqlite", "TypeScript com Electron", "Python com FastAPI", "Flutter", "Godot com GDScript", "Node sem dependências", "Tauri com Leptos"]
TOPICS = ["cache de disco", "fila de tarefas", "parser de configuração", "renderização por tiles", "sincronização offline", "geração de assets", "painel de métricas", "editor de nós"]


def seeded_filler():
    r = random.Random(7)
    for name in FILLER:
        stack, topic = r.choice(STACKS), r.choice(TOPICS)
        other = r.choice(TOPICS)
        yield name, [
            ("decision", f"Stack do {name}", f"O {name} usa {stack} para {topic}.", f"anota: o {name} usa {stack} para {topic}"),
            ("episode", f"Estado do {name}", f"Parou no {name} em {other}; o resto ficou para depois.", f"anota: parei no {name} em {other}, o resto ficou para depois"),
            ("concept", f"Lição do {name}", f"No {name}, {topic} ficou simples depois de separar {other}.", f"anota: no {name}, {topic} ficou simples depois de separar {other}"),
        ]


# (id, pasta, pedido, grupos, verificações). Cada verificação é (nome, regex esperada, regex proibida).
CASES = [
    ("new-rust", "renomeia-fotos", "vou começar um CLI em Rust pra renomear fotos por data. me dá o esqueleto de main.rs e do tipo de erro, bem curto.",
     [("erros como eu uso", r"thiserror", None), ("binário com anyhow", r"anyhow", None), ("sem unwrap", r"fn main", r"\.unwrap\(\)"),
      ("comentário em português", r"//[^\n]*(data|arquivo|foto|erro|nucleo|logica|borda|pura)", None)]),
    ("cross-compile", "ponte-pg", "tô usando pg_query_go num CLI novo e a compilação cruzada pra linux-arm64 no CI tá falhando. já passei por isso? o que eu fiz?",
     [("lembra a solução", r"zig", None), ("diz de onde veio", r"peneira", None)]),
    ("resume", "mina-infinita", "voltei pra cá depois de três semanas. onde parei e qual o próximo passo?",
     [("onde parou", r"solver", None), ("próximo passo", r"proptest|hud", None)]),
    ("why-no-build", "carta-rapida", "quero adicionar Vite e Tailwind aqui pra ir mais rápido. faz sentido?",
     [("lembra o motivo", r"godot", None), ("core sem DOM", r"dom\b", None)]),
    ("no-bleed", "vitrola", "qual é a regra do binário de hook desse projeto? posso usar tokio nele?",
     [("não atribui a regra à vitrola", r"trilho|nao (ha|tem|encontrei|achei)|nenhum", r"binario de hook (da vitrola|deste projeto|desse projeto) (fica )?sem tokio")]),
    ("review", "estudo-go", "me faz 3 perguntas de revisão só sobre o que eu já estudei de Go.",
     [("goroutine", r"goroutine", None), ("errors.Is", r"errors\.is", None), ("append", r"append|slices\.clone", None)]),
    ("template", "renomeia-fotos", "o que eu repito entre projetos que poderia virar um template pra projeto novo?",
     [("núcleo puro", r"functional core|logica pura", None), ("adr", r"adr", None), ("HOME em teste", r"\bhome\b", None)]),
    ("unknown", "foguete", "em que pé está o projeto foguete?",
     [("não inventa", r"nao (tenho|encontrei|achei|ha|sei)|nenhum|sem registro", None)]),
    ("teach", "ladrilho", "descobri agora: o hydrate do Stencil quebrava no Vite, precisa de optimizeDeps.exclude. guarda pra próxima.",
     []),
    ("reuse", "outro-ds", "o hydrate do Stencil quebrou no Vite aqui. lembra o que era?",
     [("reaproveita a lição de outro projeto", r"optimizedeps", None)]),
]


def fold(s):
    return "".join(c for c in unicodedata.normalize("NFD", s.lower()) if not unicodedata.combining(c))


def paths(base):
    return {"brain": f"{base}/brain", "home": f"{base}/home", "projects": f"{base}/projects", "out": f"{base}/runs.json"}


def env_of(p):
    return {**os.environ, "HOME": p["home"]}


def hook(p, sid, cwd, event, name, **extra):
    payload = {"session_id": sid, "cwd": cwd, "transcript_path": "", "hook_event_name": name, **extra}
    r = subprocess.run([BIN, "hook", event, "--brain", p["brain"]], input=json.dumps(payload), capture_output=True, text=True, timeout=10, env=env_of(p))
    try:
        return (json.loads(r.stdout or "{}").get("hookSpecificOutput") or {}).get("additionalContext", "") or ""
    except json.JSONDecodeError:
        return ""


def teach(p, project, facts):
    """Uma sessão sem IA: o hook registra a frase e o MCP grava, como depois de uma conversa."""
    cwd = f"{p['projects']}/{project}"
    os.makedirs(cwd, exist_ok=True)
    sid = f"seed-{project}-{uuid.uuid4().hex[:6]}"
    mcp = subprocess.Popen([BIN, "mcp", "--brain", p["brain"], "--client", "claude-code", "--session", sid, "--cwd", cwd],
                           stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, env=env_of(p))
    hook(p, sid, cwd, "session-start", "SessionStart", source="startup")
    outcomes = []
    for i, (kind, title, summary, said) in enumerate(facts):
        hook(p, sid, cwd, "prompt", "UserPromptSubmit", prompt=said)
        args = {"key": f"{project}-{i}", "type": kind, "title": title, "summary": summary, "quote": said,
                "origin_kind": "request"}
        call = {"jsonrpc": "2.0", "id": i + 1, "method": "tools/call", "params": {"name": "memory_write", "arguments": args}}
        mcp.stdin.write(json.dumps(call) + "\n")
        mcp.stdin.flush()
        reply = json.loads(mcp.stdout.readline())
        body = reply.get("result", {}).get("structuredContent") or json.loads(reply["result"]["content"][0]["text"])
        outcomes.append((body.get("outcome"), body.get("reason"), bool(body.get("warnings"))))
    hook(p, sid, cwd, "session-end", "SessionEnd", reason="exit")
    mcp.stdin.close()
    mcp.wait(timeout=10)
    return outcomes


def seed(base):
    p = paths(base)
    os.makedirs(p["home"], exist_ok=True)
    os.makedirs(p["projects"], exist_ok=True)
    subprocess.run([BIN, "init", p["brain"], "--name", "Estudos do Rafa"], check=True, capture_output=True, env=env_of(p))
    notes = f"{base}/notes-seed"
    os.makedirs(notes, exist_ok=True)
    tally = {}
    for name, about in REPO.items():
        open(f"{notes}/{name}.md", "w", encoding="utf8").write(f"# {name}\n\n{about}\n")
    for name, facts in seeded_filler():
        open(f"{notes}/{name}.md", "w", encoding="utf8").write(f"# {name}\n\n{facts[0][2]}\n\n{facts[1][2]}\n")
    subprocess.run([BIN, "import-markdown", notes, "--brain", p["brain"]], check=True, capture_output=True, env=env_of(p))
    def count(items):
        for o in items:
            key = o[0] + ("/agent" if o[2] else "") + (f":{o[1]}" if o[1] else "")
            tally[key] = tally.get(key, 0) + 1
    count(teach(p, "geral", [("preference", t, s, q) for t, s, q in GENERAL]))
    for project, facts in PROJECTS.items():
        count(teach(p, project, facts))
    for project, facts in seeded_filler():
        count(teach(p, project, facts))
    print("seed", tally)
    print(subprocess.run([BIN, "doctor", "--brain", p["brain"]], capture_output=True, text=True, env=env_of(p)).stdout)


def repo_context(folder):
    about = REPO.get(folder)
    return f"<contexto-do-repositorio>\n{about}\n</contexto-do-repositorio>\n" if about else ""


def ask(p, folder, msg, arm):
    cwd = f"{p['projects']}/{folder}"
    os.makedirs(cwd, exist_ok=True)
    sid = f"{arm}-{folder}-{uuid.uuid4().hex[:6]}"
    prompt, brief = repo_context(folder) + msg, ""
    cmd = ["claude", "-p", "--model", "sonnet", "--setting-sources", "", "--no-session-persistence", "--tools", "",
           "--max-budget-usd", "0.40", "--output-format", "stream-json", "--verbose", "--strict-mcp-config"]
    if arm == "omnimind":
        cfg = f"{p['projects']}/mcp-{uuid.uuid4().hex[:6]}.json"
        json.dump({"mcpServers": {"omnimind": {"command": BIN, "env": {"HOME": p["home"]},
                  "args": ["mcp", "--brain", p["brain"], "--client", "claude-code", "--session", sid, "--cwd", cwd]}}}, open(cfg, "w"))
        cmd += ["--mcp-config", cfg, "--allowedTools", TOOLS]
        brief = hook(p, sid, cwd, "session-start", "SessionStart", source="startup")
        ctx = hook(p, sid, cwd, "prompt", "UserPromptSubmit", prompt=msg)
        prompt = "\n".join([f"<system-context>\n{c}\n</system-context>" for c in (brief, ctx) if c] + [prompt])
    else:
        cmd += ["--mcp-config", '{"mcpServers":{}}']
        # Sem este aviso o modelo escreve chamadas de ferramenta como texto e nem responde.
        prompt += "\n\n(Nesta sessão não há ferramentas nem acesso a arquivos: responda direto, só com o que você sabe.)"
    t0 = time.time()
    run = subprocess.run(cmd + ["--", prompt], cwd=cwd, capture_output=True, text=True, timeout=170)
    secs = round(time.time() - t0, 1)
    reply, tools, result = "", [], None
    for line in run.stdout.splitlines():
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        if ev.get("type") == "assistant":
            for b in ev["message"].get("content", []):
                if b.get("type") == "text":
                    reply += b["text"]
                elif b.get("type") == "tool_use":
                    tools.append(b["name"].split("__")[-1])
        elif ev.get("type") == "result":
            result = ev
    if arm == "omnimind":
        hook(p, sid, cwd, "stop", "Stop", last_assistant_message=reply)
        hook(p, sid, cwd, "session-end", "SessionEnd", reason="exit")
    u = (result or {}).get("usage") or {}
    return {"arm": arm, "reply": reply, "tools": tools, "seconds": secs, "cost": (result or {}).get("total_cost_usd") or 0.0,
            "input_tokens": (u.get("input_tokens") or 0) + (u.get("cache_read_input_tokens") or 0) + (u.get("cache_creation_input_tokens") or 0),
            "output_tokens": u.get("output_tokens") or 0, "brief_chars": len(brief),
            "models": list(((result or {}).get("modelUsage") or {}).keys()), "error": None if result else run.stderr[-200:]}


def verdicts(reply, checks):
    f = fold(reply)
    return {name: bool(re.search(want, f)) and not (ban and re.search(ban, f)) for name, want, ban in checks}


def load(p):
    return json.load(open(p["out"])) if os.path.exists(p["out"]) else {"spent": 0.0, "rows": []}


def run(base, only):
    p = paths(base)
    for cid, folder, msg, checks in CASES:
        if only and cid not in only:
            continue
        for arm in os.environ.get("ARMS", "omnimind,control").split(","):
            if cid == "teach" and arm == "control":
                continue
            log = load(p)
            if log["spent"] + 0.05 > CAP:
                print("budget stop")
                return
            if any(r["case"] == cid and r["arm"] == arm for r in log["rows"]):
                continue
            r = ask(p, folder, msg, arm)
            r["case"], r["folder"], r["user"] = cid, folder, msg
            r["checks"] = verdicts(r["reply"], checks)
            log["rows"].append(r)
            log["spent"] = round(log["spent"] + r["cost"], 4)
            json.dump(log, open(p["out"], "w"), ensure_ascii=False, indent=1)
            print(f"\n### {cid} [{arm}] {r['seconds']}s US$ {r['cost']:.4f} tools={r['tools']} checks={r['checks']}\n{r['reply']}", flush=True)
    print("spent", load(p)["spent"])


def report(base):
    log = load(paths(base))
    rows = log["rows"]
    print(f"{'caso':<15}{'omnimind':<12}{'controle':<12}US$ O   US$ C   tokens O/C")
    totals = {"omnimind": [0, 0], "control": [0, 0]}
    for cid, _, _, checks in CASES:
        o = next((r for r in rows if r["case"] == cid and r["arm"] == "omnimind"), None)
        c = next((r for r in rows if r["case"] == cid and r["arm"] == "control"), None)
        if not o or not checks:
            continue
        line = []
        for arm, r in (("omnimind", o), ("control", c)):
            ok = sum(r["checks"].values()) if r else 0
            totals[arm][0] += ok
            totals[arm][1] += len(checks)
            line.append(f"{ok}/{len(checks)}")
        cc = c["cost"] if c else 0
        ct = c["input_tokens"] if c else 0
        print(f"{cid:<15}{line[0]:<12}{line[1]:<12}{o['cost']:.3f}  {cc:.3f}  {o['input_tokens']}/{ct}")
    print("total", totals, "spent", log["spent"], "models", {m for r in rows for m in r["models"]})
    print("resumo inicial (caracteres)", sorted({r["brief_chars"] for r in rows if r["arm"] == "omnimind"}))


if __name__ == "__main__":
    {"seed": lambda: seed(sys.argv[2]), "run": lambda: run(sys.argv[2], sys.argv[3:]), "report": lambda: report(sys.argv[2])}[sys.argv[1]]()
