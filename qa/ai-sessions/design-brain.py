#!/usr/bin/env python3
"""O mesmo pedido de layout, com e sem um cofre de design conectado.

Os dois grupos recebem o mesmo texto e o mesmo print. O grupo com cofre tem o MCP, o resumo inicial
e uma frase a mais pedindo para consultá-lo. Cada grupo roda uma vez, em processo novo.

  design-brain.py DIR_COFRE PASTA_DO_PROJETO SAÍDA with|without metrics PRINT.jpg
  design-brain.py DIR_COFRE PASTA_DO_PROJETO SAÍDA with|without graph|home

Em graph e home não há print nem frase pedindo para consultar: o cofre precisa se apresentar sozinho.
"""
import base64, json, os, re, subprocess, sys, time, uuid

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BIN = os.path.join(ROOT, "target/release/omnimind")
TOOLS = "mcp__omnimind__memory_search,mcp__omnimind__memory_read,mcp__omnimind__memory_write,mcp__omnimind__memory_agenda"

BRIEF = """Você é design lead de um estúdio pequeno e assina esta tela. O print anexo é a tela Métricas do Omnimind, um app de desktop que dá memória às IAs de quem programa.

Recrie essa tela em um único arquivo HTML, com o mesmo layout geral: barra lateral, título com seletor de período, quatro números, gráfico de leituras por dia, listas Mais lidas e Temas mais lidos. Leve o acabamento a um nível acima do print.

Inclua algo que ainda não existe: um bloco chamado "Sem precisar repetir". Ele responde à pergunta que faz a pessoa abrir esta tela: a memória está me poupando trabalho? Decida você a forma e o lugar, sem ser mais uma fileira de cartões.

Dados por período, com o seletor funcionando de verdade:
7 dias: 214 leituras, 18 memórias novas, 6 correções suas, 87% das buscas com resposta, 9 repetições evitadas.
30 dias: 1.042 leituras, 63 memórias novas, 21 correções, 81%, 37 repetições evitadas.
90 dias: 2.871 leituras, 140 memórias novas, 74 correções, 76%, 96 repetições evitadas.
Leituras por dia em 7 dias, IA e Você: seg 28/4, ter 41/6, qua 33/3, qui 37/5, sex 29/2, sáb 9/0, dom 14/3. Nos outros períodos, gere séries coerentes com o total.
Mais lidas: Deploy em duas etapas 31; Fila síncrona no Cuscuz 24; Formato de commit 19; Janela de deploy da Rota Certa 12; Reunião com a Coopcarga 7.
Temas: Rota Certa 92; Arquitetura 54; Preferências 31; Estudos de Go 22; Design 15.
Repetições evitadas nesta semana: "usar ureq, sem tokio" 4 vezes; "migração antes da aplicação" 3 vezes; "commit no formato tipo(escopo)" 2 vezes.

Regras: arquivo único, com CSS e JS embutidos; nada externo além de fontes do Google Fonts; tema escuro; desenhado para uma janela de 1440 por 900, sem rolagem horizontal; texto em português do Brasil. Foco de teclado visível e prefers-reduced-motion respeitado.

O critério: alguém que usa ferramentas de trabalho o dia inteiro olha e acredita que um designer experiente fez isso à mão. Não pode ter cara de gerado por IA.

Você não tem acesso aos arquivos do projeto. Entregue só o HTML, em um bloco ```html, com no máximo 280 linhas, sem explicação antes ou depois."""

COMMON = """Você é design lead de um estúdio pequeno e assina esta tela. O Omnimind é um app de desktop, local, que dá memória às IAs de quem programa: a pessoa escreve notas em Markdown, e a IA guarda memórias entre uma sessão e outra. Tudo fica numa pasta no computador da pessoa, chamada cofre.

Você não recebeu nenhum print do produto. Decida a aparência com o que souber.

"""

CLOSING = """

Regras: arquivo único, com CSS e JS embutidos; nada externo além de fontes do Google Fonts; tema escuro; desenhado para uma janela de 1440 por 900, sem rolagem da página; texto em português do Brasil. Foco de teclado visível e prefers-reduced-motion respeitado.

O critério: alguém que usa ferramentas de trabalho o dia inteiro olha e acredita que um designer experiente fez isso à mão. Não pode ter cara de gerado por IA.

Você não tem acesso aos arquivos do projeto. Entregue só o HTML, em um bloco ```html, com no máximo 240 linhas, sem explicação antes ou depois."""

SCREENS = {
    "graph": COMMON + """Desenhe a tela Grafo. Ela existe para a pessoa ver o cofre inteiro de uma vez e notar a IA trabalhando nele. A navegação do app tem Notas, Memórias, Grafo, Atividade, Métricas, Perfil e Ajustes, com o Grafo ativo.

O grafo tem duas formas, que a pessoa alterna:
Feixes: os itens ficam num círculo, agrupados por tema, e as ligações entre temas viram feixes curvos que passam pelo centro, sem virar um novelo.
Raízes: uma árvore cresce do centro, com um ramo por tema, e os ramos engrossam com o uso.

Filtro: Tudo, Notas, Memórias. Rodapé com a contagem de itens e ligações.
Gere 180 itens por código, com semente fixa, em sete temas: Rota Certa, Arquitetura, Preferências, Estudos de Go, Design, Reuniões e Ideias. Desenhe em canvas ou SVG, com abertura animada.
Ao passar o mouse, o item acende as ligações e mostra uma ficha com título, tipo e "lida 12 vezes em 30 dias". A cada poucos segundos, simule uma leitura da IA percorrendo uma ligação até um item.
Clicar num item abre um painel de detalhe à direita.""" + CLOSING,
    "home": COMMON + """Desenhe a tela inicial, que aparece uma só vez: a pessoa abriu o Omnimind e ainda não tem cofre. Sem cofre não há navegação. Esta tela é tudo o que existe.

Em cinco segundos a pessoa precisa entender o que o produto faz e o que clicar. Explique as três palavras do produto: Nota, Memória e Cofre. As saídas são Começar, Abrir uma pasta e Ver um exemplo.

Dê à tela uma ideia visual que mostre memória crescendo, feita em canvas ou SVG e gerada por código, sem imagens. Ela não usa dado real de ninguém. Se a pessoa não fizer nada, a tela deve continuar calma.""" + CLOSING,
}

CONSULT = "\n\nO cofre de design está conectado: consulte-o antes de desenhar e siga o que encontrar."


def hook(brain, home, sid, cwd, event, name, **extra):
    payload = {"session_id": sid, "cwd": cwd, "transcript_path": "", "hook_event_name": name, **extra}
    r = subprocess.run([BIN, "hook", event, "--brain", brain], input=json.dumps(payload), capture_output=True, text=True,
                       timeout=10, env={**os.environ, "HOME": home})
    try:
        return (json.loads(r.stdout or "{}").get("hookSpecificOutput") or {}).get("additionalContext", "") or ""
    except json.JSONDecodeError:
        return ""


def main():
    brain, cwd, out, arm, screen = sys.argv[1:6]
    shot = sys.argv[6] if len(sys.argv) > 6 else None
    home = os.environ["HOME"]
    sid = f"design-{screen}-{arm}-{uuid.uuid4().hex[:6]}"
    budget = "0.40"
    cmd = ["claude", "-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose", "--model", "sonnet",
           "--setting-sources", "", "--no-session-persistence", "--tools", "", "--max-budget-usd", budget, "--strict-mcp-config"]
    text = BRIEF if screen == "metrics" else SCREENS[screen]
    if arm == "with":
        cfg = f"{out}/mcp-{screen}.json"
        json.dump({"mcpServers": {"omnimind": {"command": BIN, "env": {"HOME": home},
                  "args": ["mcp", "--brain", brain, "--client", "claude-code", "--session", sid, "--cwd", cwd]}}}, open(cfg, "w"))
        cmd += ["--mcp-config", cfg, "--allowedTools", TOOLS]
        first = hook(brain, home, sid, cwd, "session-start", "SessionStart", source="startup")
        text = f"<system-context>\n{first}\n</system-context>\n{text}{CONSULT if screen == 'metrics' else ''}"
    else:
        cmd += ["--mcp-config", '{"mcpServers":{}}']
    content = [{"type": "text", "text": text}]
    if shot:
        image = base64.b64encode(open(shot, "rb").read()).decode()
        content.insert(0, {"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": image}})
    msg = {"type": "user", "message": {"role": "user", "content": content}}
    # O ambiente real fica de fora: a sessão precisa da conta, mas não lê configuração nenhuma.
    env = {**os.environ, "HOME": os.environ.get("REAL_HOME", home)}
    t0 = time.time()
    p = subprocess.Popen(cmd, cwd=cwd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, env=env)
    p.stdin.write(json.dumps(msg) + "\n")
    p.stdin.flush()
    reply, tools, result = "", [], None
    for line in p.stdout:
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        if ev.get("type") == "assistant":
            for b in ev["message"].get("content", []):
                if b.get("type") == "text":
                    reply += b["text"]
                elif b.get("type") == "tool_use":
                    tools.append({"tool": b["name"].split("__")[-1], "input": b["input"]})
        elif ev.get("type") == "result":
            result = ev
            break
    p.stdin.close()
    p.wait(timeout=30)
    if arm == "with":
        hook(brain, home, sid, cwd, "session-end", "SessionEnd", reason="exit")
    found = re.search(r"```html\s*(.*?)```", reply, re.S) or re.search(r"(<!doctype html.*)", reply, re.S | re.I)
    html = found.group(1).strip() if found else ""
    open(f"{out}/{screen}-{arm}.html", "w", encoding="utf8").write(html)
    r = result or {}
    report = {"screen": screen, "arm": arm, "seconds": round(time.time() - t0, 1), "cost": r.get("total_cost_usd"), "subtype": r.get("subtype"),
              "turns": r.get("num_turns"), "models": list((r.get("modelUsage") or {}).keys()), "tools": tools, "html_lines": len(html.splitlines()),
              "output_tokens": (r.get("usage") or {}).get("output_tokens"), "prose": reply[:300] if not found else reply[:found.start()][:300]}
    json.dump(report, open(f"{out}/{screen}-{arm}.json", "w"), ensure_ascii=False, indent=1)
    print(json.dumps(report, ensure_ascii=False)[:1400])


main()
