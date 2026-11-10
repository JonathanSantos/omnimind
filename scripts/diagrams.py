#!/usr/bin/env python3
"""Desenha os diagramas do README em SVG, nos temas claro e escuro.

Cores vêm de `crates/app/src/theme.rs` e fontes de `assets/fonts`: mudar o design system muda as
imagens. O texto vira contorno, porque um SVG dentro de `<img>` não carrega fonte externa.

  python3 scripts/diagrams.py
"""
import math, os, random, re
from xml.sax.saxutils import escape

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "docs/brand/diagrams")


def tokens():
    src = open(os.path.join(ROOT, "crates/app/src/theme.rs"), encoding="utf8").read()
    parts = re.split(r"\n\s*bg: Color::hex", src)[1:3]
    themes = []
    for part in parts:
        text = "bg: Color::hex" + part
        t = {k: "#" + v for k, v in re.findall(r"(\w+): Color::hex\(0x([0-9A-Fa-f]{6})\)", text)}
        t["series"] = ["#" + c for c in re.findall(r"0x([0-9A-Fa-f]{6})", text[text.index("themes:"):].split("]")[0])]
        themes.append(t)
    return {"light": themes[0], "dark": themes[1]}


class Face:
    """Um peso de uma família, com contornos e larguras."""

    def __init__(self, path, weight=None):
        font = TTFont(path)
        if weight and "fvar" in font:
            font = instancer.instantiateVariableFont(font, {"wght": weight}, inplace=False)
        self.glyphs = font.getGlyphSet()
        self.cmap = font.getBestCmap()
        self.hmtx = font["hmtx"]
        self.upem = font["head"].unitsPerEm

    def width(self, s, size):
        return sum(self.hmtx[self.cmap.get(ord(c), ".notdef")][0] for c in s) * size / self.upem

    def path(self, s, x, y, size):
        k = size / self.upem
        pen = SVGPathPen(self.glyphs, ntos=lambda v: f"{v:.1f}".rstrip("0").rstrip("."))
        for c in s:
            name = self.cmap.get(ord(c), ".notdef")
            self.glyphs[name].draw(TransformPen(pen, (k, 0, 0, -k, x, y)))
            x += self.hmtx[name][0] * k
        return pen.getCommands()


FONTS = {
    "sans": Face(os.path.join(ROOT, "assets/fonts/FamiljenGrotesk.ttf"), 400),
    "strong": Face(os.path.join(ROOT, "assets/fonts/FamiljenGrotesk.ttf"), 600),
    "mono": Face(os.path.join(ROOT, "assets/fonts/SometypeMono.ttf"), 400),
}


class Canvas:
    def __init__(self, name, w, h, theme, title, desc):
        self.name, self.w, self.h, self.t, self.title, self.desc = name, w, h, theme, title, desc
        self.style = ""
        self.items = [f'<rect x=".5" y=".5" width="{w - 1}" height="{h - 1}" rx="12" fill="{theme["bg"]}" stroke="{theme["line"]}"/>']

    def text(self, s, x, y, size=18, face="sans", color=None, anchor="start"):
        color = color or self.t["ink"]
        for i, line in enumerate(s.split("\n")):
            f = FONTS[face]
            at = x - {"start": 0, "middle": f.width(line, size) / 2, "end": f.width(line, size)}[anchor]
            self.items.append(f'<path fill="{color}" d="{f.path(line, at, y + i * size * 1.34, size)}"/>')

    def box(self, x, y, w, h, title, sub=None, fill=None, stroke=None, ink=None, center=False, size=18, sub_color=None, r=8):
        t = self.t
        self.items.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill or t["panel"]}" stroke="{stroke or t["line"]}"/>')
        lines = 1 + title.count("\n") + (1 + sub.count("\n") if sub else 0)
        block = lines * size * 1.3
        top = y + (h - block) / 2 + size * 0.95
        tx, anchor = (x + w / 2, "middle") if center else (x + 18, "start")
        self.text(title, tx, top, size, "strong", ink or t["ink"], anchor)
        if sub:
            self.text(sub, tx, top + (1 + title.count("\n")) * size * 1.34, size - 2.5, "sans", sub_color or t["sub"], anchor)

    def line(self, d, color=None, width=1.5, dash=None, opacity=None):
        extra = (f' stroke-dasharray="{dash}"' if dash else "") + (f' stroke-opacity="{opacity}"' if opacity else "")
        self.items.append(f'<path d="{d}" fill="none" stroke="{color or self.t["mute"]}" stroke-width="{width}" stroke-linecap="round"{extra}/>')

    def head(self, x, y, angle, color=None, size=9):
        a, b = angle + 2.62, angle - 2.62
        pts = f"M{x:.1f} {y:.1f}L{x + size * math.cos(a):.1f} {y + size * math.sin(a):.1f}L{x + size * math.cos(b):.1f} {y + size * math.sin(b):.1f}Z"
        self.items.append(f'<path d="{pts}" fill="{color or self.t["mute"]}"/>')

    def arrow(self, x1, y1, x2, y2, color=None, width=1.5, dash=None):
        self.line(f"M{x1} {y1}L{x2} {y2}", color, width, dash)
        self.head(x2, y2, math.atan2(y2 - y1, x2 - x1), color)

    def curve(self, x1, y1, x2, y2, color=None, width=1.5, horizontal=True, tip=True, opacity=None, dash=None):
        if horizontal:
            m = (x2 - x1) * 0.5
            d = f"M{x1} {y1}C{x1 + m} {y1} {x2 - m} {y2} {x2} {y2}"
            angle = 0 if x2 >= x1 else math.pi
        else:
            m = (y2 - y1) * 0.5
            d = f"M{x1} {y1}C{x1} {y1 + m} {x2} {y2 - m} {x2} {y2}"
            angle = math.pi / 2 if y2 >= y1 else -math.pi / 2
        self.line(d, color, width, dash, opacity)
        if tip:
            self.head(x2, y2, angle, color)

    def dot(self, x, y, r, fill, stroke=None):
        self.items.append(f'<circle cx="{x}" cy="{y}" r="{r}" fill="{fill}"' + (f' stroke="{stroke}"' if stroke else "") + "/>")

    def number(self, x, y, n, on=False):
        t = self.t
        self.dot(x, y, 15, t["acc"] if on else t["panel"], t["acc"] if on else t["line"])
        self.text(str(n), x, y + 5, 14, "mono", t["bg"] if on else t["sub"], "middle")

    def save(self):
        body = "".join(self.items)
        svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {self.w} {self.h}" width="{self.w}" height="{self.h}" role="img" aria-labelledby="t d">'
               f'<title id="t">{escape(self.title)}</title><desc id="d">{escape(self.desc)}</desc>{self.style}{body}</svg>')
        side = "dark" if self.t is THEMES["dark"] else "light"
        with open(os.path.join(OUT, f"{self.name}-{side}.svg"), "w", encoding="utf8") as f:
            f.write(svg)


def first_session(t):
    c = Canvas("first-session", 1280, 250, t, "Da instalação à primeira sessão", "Instalar, começar, conectar, abrir sessão e lembrar.")
    steps = [("Instalar", "Arraste para\nAplicativos"), ("Começar", "O cofre nasce\nem ~/Omnimind"), ("Conectar", "Escolha o projeto\nem Ajustes"),
             ("Abrir sessão", "No projeto\nconectado"), ("Lembrar", "A IA já chega\ncom contexto")]
    gap = 1280 / len(steps)
    xs = [gap * i + gap / 2 for i in range(len(steps))]
    c.line(f"M{xs[0]} 76L{xs[-1]} 76", t["line"], 2)
    c.line(f"M{xs[-2]} 76L{xs[-1]} 76", t["acc"], 2)
    for i, (x, (a, b)) in enumerate(zip(xs, steps)):
        c.number(x, 76, i + 1, i == len(steps) - 1)
        c.text(a, x, 128, 21, "strong", None, "middle")
        c.text(b, x, 158, 17, "sans", t["sub"], "middle")
    c.save()


def session(t):
    c = Canvas("session", 1180, 980, t, "Uma sessão por dentro", "Hooks entregam o resumo e guardam a frase literal; o servidor MCP busca e grava no cofre.")
    lanes = [("Você", 120), ("Claude Code", 370), ("Hooks", 620), ("Servidor MCP", 830), ("Cofre", 1050)]
    x = dict(lanes)
    for name, lx in lanes:
        c.box(lx - 86, 34, 172, 52, name, center=True, size=18, fill=t["side"])
        c.line(f"M{lx} 86L{lx} 930", t["line"], 1.5, "2 7")
    phases = [("início", 112, 244), ("sua mensagem", 262, 502), ("trabalho", 520, 760), ("fim", 778, 924)]
    for name, top, bottom in phases:
        c.items.append(f'<rect x="20" y="{top}" width="1140" height="{bottom - top}" rx="8" fill="{t["side"]}" fill-opacity=".55"/>')
        c.text(name, 36, top + 24, 14, "mono", t["mute"])

    def msg(a, b, y, label, color=None, back=False, strong=False):
        x1, x2 = x[a], x[b]
        c.dot(x1, y, 3.5, color or t["mute"])
        c.arrow(x1, y, x2 + (8 if x2 < x1 else -8), y, color, 2 if strong else 1.5, "5 5" if back else None)
        mid = (x1 + x2) / 2
        c.text(label, mid, y - 11, 16.5, "strong" if strong else "sans", color if strong else t["sub"], "middle")

    msg("Claude Code", "Hooks", 158, "SessionStart")
    msg("Hooks", "Cofre", 192, "lê memórias e notas")
    msg("Hooks", "Claude Code", 226, "resumo inicial", back=True)
    msg("Você", "Claude Code", 318, "“não, aqui usamos pnpm”")
    msg("Claude Code", "Hooks", 362, "UserPromptSubmit")
    msg("Hooks", "Cofre", 412, "grava sua frase literal, sem IA", t["acc"], strong=True)
    msg("Hooks", "Claude Code", 470, "aviso de correção", back=True)
    msg("Claude Code", "Servidor MCP", 572, "memory_search")
    msg("Servidor MCP", "Claude Code", 618, "resultados com origem", back=True)
    msg("Claude Code", "Servidor MCP", 672, "memory_write, com citação")
    msg("Servidor MCP", "Cofre", 730, "portão, grava", t["acc"], strong=True)
    msg("Claude Code", "Hooks", 830, "Stop e SessionEnd")
    msg("Hooks", "Cofre", 884, "citações e fim da sessão")
    c.save()


def correction(t):
    c = Canvas("correction", 900, 1090, t, "De correção a memória", "A frase é gravada sem IA, passa por três verificações e volta no resumo da próxima sessão.")
    cx = 300
    c.box(cx - 170, 36, 340, 72, "Você corrige", '“não, aqui usamos pnpm”', center=True, fill=t["side"], size=19)
    c.arrow(cx, 108, cx, 146)
    c.box(cx - 170, 150, 340, 72, "O hook grava a frase", "literal, sem usar IA", center=True, stroke=t["acc"], size=19)
    c.arrow(cx, 222, cx, 260)
    c.box(cx - 170, 264, 340, 72, "A IA destila", "e cita o que você disse", center=True, size=19)
    checks = [(382, "Tem segredo?", "Recusa", "Nada é gravado", t["warn"]), (526, "Parece instrução\nmaliciosa?", "Quarentena", "Fica para você revisar", t["series"][1])]
    c.arrow(cx, 336, cx, 378)
    for y, q, out, sub, color in checks:
        c.box(cx - 170, y, 340, 88, q, center=True, fill=t["side"], stroke=t["mute"], size=19)
        c.line(f"M{cx + 170} {y + 44}L{590} {y + 44}", color, 1.5)
        c.head(600, y + 44, 0, color)
        c.text("sim", 500, y + 34, 15, "mono", color, "middle")
        c.box(604, y + 2, 262, 84, out, sub, stroke=color, ink=color, size=18)
        c.text("não", cx + 14, y + 122, 15, "mono", t["sub"])
        c.arrow(cx, y + 88, cx, y + 140)
    c.box(cx - 170, 670, 340, 88, "A citação é sua\ne sustenta o texto?", center=True, fill=t["side"], stroke=t["mute"], size=19)
    c.curve(cx - 70, 758, 200, 846, t["acc"], 2, horizontal=False)
    c.curve(cx + 70, 758, 600, 846, t["mute"], 1.5, horizontal=False)
    c.text("sim", 168, 806, 15, "mono", t["acc"], "end")
    c.text("não", 500, 802, 15, "mono", t["sub"])
    c.box(60, 850, 300, 84, "Memória sua", "autoridade da pessoa", stroke=t["acc"], ink=t["acc"], center=True, size=19)
    c.box(450, 850, 300, 84, "Memória da IA", "com aviso, para revisar", center=True, size=19)
    c.curve(210, 934, 400, 984, t["acc"], 2, horizontal=False, tip=False)
    c.curve(600, 934, 400, 984, t["mute"], 1.5, horizontal=False)
    c.line("M400 984L400 990", t["acc"], 2)
    c.box(170, 990, 460, 66, "Volta no resumo da próxima sessão", center=True, fill=t["sel"], size=19)
    c.save()


def projects(t):
    c = Canvas("projects", 1280, 620, t, "Vários projetos, uma memória", "O resumo traz o projeto atual, preferências e títulos de outros projetos; a busca alcança todos.")
    names = ["rota-certa", "cuscuz", "estudo-go", "trilho", "vitrola", "mina-infinita", "ladrilho"]
    c.text("Cofre", 62, 62, 16, "mono", t["mute"])
    top, row = 84, 66
    ys = [top + i * row for i in range(len(names))]
    layers = [("Quem é você", "vale em qualquer projeto", 58, 86, t["acc"]), ("Preferências gerais", "vale em qualquer projeto", 158, 86, t["acc"]),
              ("Tudo do projeto atual", "rota-certa", 258, 104, t["acc"]), ("Até 12 títulos de outros projetos", "marcados: não valem aqui sem confirmar", 376, 104, t["line"])]
    bx = 520
    for i, (name, y) in enumerate(zip(names, ys)):
        color = t["series"][i % len(t["series"])]
        if i:
            for k in range(3):
                c.curve(262, y + 26 + (k - 1) * 5, bx, 412 + i * 6, color, 1.2, tip=False, opacity=".5")
    for k in range(5):
        c.curve(262, ys[0] + 26 + (k - 2) * 4, bx, 296 + k * 8, t["series"][0], 2, tip=False)
    for i, (name, y) in enumerate(zip(names, ys)):
        color = t["series"][i % len(t["series"])]
        on = i == 0
        c.items.append(f'<rect x="56" y="{y}" width="206" height="52" rx="8" fill="{t["sel"] if on else t["panel"]}" stroke="{color if on else t["line"]}"/>')
        c.dot(80, y + 26, 6, color)
        c.text(name, 98, y + 32, 18, "strong" if on else "sans", t["ink"] if on else t["sub"])
    c.text("Resumo inicial", bx, 40, 16, "mono", t["mute"])
    for title, sub, y, h, stroke in layers:
        c.box(bx, y, 400, h, title, sub, stroke=stroke, size=19)
    c.line("M920 428L990 428", t["mute"], 1.5, "5 5")
    c.head(1000, 428, 0)
    c.box(1004, 362, 226, 132, "memory_search", "“já vi algo\nparecido?”\nalcança todos", center=True, fill=t["side"], size=18)
    c.text("A regra de um projeto\nnão vaza para o outro.", 1004, 544, 16, "sans", t["sub"])
    c.save()


def switch(c, x, y, on):
    t = c.t
    c.items.append(f'<rect x="{x}" y="{y}" width="56" height="30" rx="15" fill="{t["acc"] if on else t["line"]}"/>')
    c.dot(x + (41 if on else 15), y + 15, 11, t["bg"] if on else t["ink"])


def on_off(t):
    c = Canvas("on-off", 1280, 430, t, "Ligar e desligar", "Conectar liga o projeto; o interruptor alterna; Remover tira da lista; a pasta ausente é detectada.")
    c.dot(70, 180, 8, t["mute"])
    c.arrow(78, 180, 196, 180)
    c.text("Conectar\nprojeto", 136, 130, 16, "sans", t["sub"], "middle")
    c.box(200, 112, 320, 136, "Ligado", "Hooks e MCP no projeto", stroke=t["acc"], size=21)
    switch(c, 440, 130, True)
    c.box(700, 112, 320, 136, "Desligado", "Projeto limpo, cofre igual", size=21)
    switch(c, 940, 130, False)
    c.curve(520, 156, 700, 156, t["sub"], 1.5)
    c.curve(700, 206, 520, 206, t["sub"], 1.5)
    c.text("interruptor", 610, 141, 15, "mono", t["sub"], "middle")
    c.text("interruptor", 610, 234, 15, "mono", t["sub"], "middle")
    c.box(200, 318, 320, 76, "Pasta não encontrada", stroke=t["warn"], ink=t["warn"], size=19)
    c.arrow(360, 248, 360, 314, t["warn"])
    c.text("a pasta sumiu", 376, 290, 15, "mono", t["warn"])
    c.box(1080, 150, 150, 60, "Fora da lista", center=True, fill=t["side"], size=17)
    c.arrow(1020, 180, 1076, 180)
    c.curve(520, 356, 860, 240, t["mute"], 1.5, tip=False)
    c.curve(860, 240, 1155, 214, t["mute"], 1.5)
    c.text("Remover", 1048, 170, 15, "mono", t["sub"], "middle")
    c.text("Remover", 720, 318, 15, "mono", t["sub"], "middle")
    c.text("O estado vem dos arquivos do projeto:\na tela não mostra uma conexão que não existe.", 880, 352, 16, "sans", t["sub"])
    c.save()


def review(t):
    c = Canvas("review", 1280, 430, t, "Conferir e desfazer", "O que a IA grava aparece em Atividade e pode seguir, ser corrigido ou ser desfeito.")
    c.box(40, 170, 220, 92, "A IA gravou algo", size=19, fill=t["side"])
    c.arrow(260, 216, 310, 216)
    c.box(314, 170, 220, 92, "Atividade", "mostra a mudança", size=19)
    c.arrow(534, 216, 584, 216)
    c.box(588, 170, 180, 92, "Está certo?", center=True, fill=t["side"], stroke=t["mute"], size=19)
    rows = [(60, "sim", "Segue valendo", "Entra nas próximas sessões", t["acc"]), (170, "em parte", "Corrigir no Perfil", "Vale na próxima sessão", t["series"][1]), (280, "não", "Desfazer", "O arquivo volta como estava", t["warn"])]
    for y, label, title, sub, color in rows:
        c.curve(768, 216, 880, y + 46, color, 1.8)
        c.box(884, y, 350, 92, title, sub, stroke=color, size=19)
        c.text(label, 824, y + 46 + (-18 if y == 170 else (-28 if y == 60 else 34)), 14, "mono", color, "middle")
    c.save()


def dream(t):
    c = Canvas("dream", 900, 880, t, "Consolidar com /sonhar", "A agenda lista o trabalho, a IA lê, grava o durável e repete até esvaziar.")
    cx, cy, r = 450, 480, 262
    c.line(f"M{cx + r} {cy}A{r} {r} 0 1 1 {cx + r - .1} {cy - 2}", t["line"], 2)
    stops = [(-90, "Agenda", "notas novas, sinais,\nduplicatas, contradições"), (0, "Ler", "cada nota inteira"), (90, "Decidir", "o que é durável"), (180, "Gravar", "com origem e citação")]
    for deg in (-45, 45, 135, 225):
        a = math.radians(deg)
        c.head(cx + r * math.cos(a), cy + r * math.sin(a), a + math.pi / 2, t["acc"], 12)
    for deg, title, sub in stops:
        a = math.radians(deg)
        x, y = cx + r * math.cos(a), cy + r * math.sin(a)
        w, h = (270, 104) if deg == -90 else (200, 84)
        c.box(x - w / 2, y - h / 2, w, h, title, sub, center=True, stroke=t["acc"] if deg == -90 else None, size=19)
    c.dot(cx, cy, 74, t["side"], t["line"])
    c.text("/sonhar", cx, cy + 7, 23, "mono", t["acc"], "middle")
    c.curve(450, 166, 450, 112, t["sub"], 1.5, horizontal=False)
    c.text("agenda vazia", 466, 146, 14, "mono", t["sub"])
    c.box(300, 28, 300, 80, "Relatório", "o que fez e o que ficou aberto", center=True, fill=t["side"], size=18)
    c.save()


def architecture(t):
    c = Canvas("architecture", 1180, 900, t, "Como funciona por dentro", "A IA fala com hooks e servidor MCP; o app e o servidor usam o mesmo núcleo; tudo termina no cofre.")
    c.text("Sua IA", 40, 54, 16, "mono", t["mute"])
    c.box(40, 72, 240, 84, "Claude Code", center=True, fill=t["side"], size=20)
    c.box(310, 72, 240, 84, "GitHub Copilot", center=True, fill=t["side"], size=20)
    c.items.append(f'<rect x="28" y="204" width="1124" height="356" rx="12" fill="{t["side"]}" fill-opacity=".55" stroke="{t["line"]}"/>')
    c.text("Omnimind", 52, 240, 16, "mono", t["mute"])
    c.box(56, 262, 300, 110, "omnimind hook", "resumo inicial e\nfrase literal no diário", size=19)
    c.box(392, 262, 300, 110, "omnimind mcp", "4 ferramentas, stdio\nvive só durante a sessão", size=19)
    c.box(830, 262, 296, 110, "Omnimind.app", "Rust nativo\nwgpu sobre Metal", size=19)
    c.box(392, 432, 734, 100, "omnimind-core", "portão de escrita · busca BM25F · diário · ciclo de vida", stroke=t["acc"], size=20)
    for x0 in (160, 430):
        c.curve(x0, 156, 206, 258, t["mute"], 1.5, horizontal=False)
        c.curve(x0, 156, 542, 258, t["mute"], 1.5, horizontal=False)
    c.arrow(542, 372, 542, 428)
    c.arrow(978, 372, 978, 428)
    c.curve(206, 372, 280, 596, t["acc"], 2, horizontal=False)
    c.text("só anexa,\nnunca bloqueia", 76, 470, 15, "mono", t["sub"])
    c.curve(760, 532, 600, 596, t["acc"], 2, horizontal=False)
    c.items.append(f'<rect x="28" y="600" width="1124" height="264" rx="12" fill="{t["panel"]}" stroke="{t["acc"]}"/>')
    c.text("Cofre, uma pasta sua", 52, 640, 20, "strong", t["acc"])
    folders = [("notes/", "suas notas em Markdown", 52), ("memories/", "um arquivo por memória", 416), (".omnimind/", "diário só de anexar,\níndice descartável", 780)]
    for name, sub, x0 in folders:
        c.box(x0, 666, 340, 110, name, sub, size=20)
    c.curve(950, 666, 1090, 372 + 8, t["sub"], 1.5, horizontal=False, dash="5 5")
    c.text("diário ao vivo", 1000, 588, 14, "mono", t["sub"], "middle")
    c.text("O disco é a verdade. Sem banco de dados, sem processo em segundo plano e sem nuvem.", 52, 824, 17, "sans", t["sub"])
    c.save()


def write_path(t):
    c = Canvas("write-path", 1280, 450, t, "O caminho de escrita", "App, CLI, MCP e hooks passam pelas mesmas oito etapas até o índice.")
    top = [("Pedido", "app, CLI, MCP"), ("Segredo", "texto bruto"), ("Idempotência", "mesma chave,\nnada muda"), ("Autoridade", "citação e\ndiário")]
    low = [("Instrução", "suspeito vai\npara quarentena"), ("Colisão", "só idêntico\nfunde"), ("Gravar", "rename atômico"), ("Diário e índice", "evento, depois\nbusca")]
    w, h, gap, x0 = 262, 118, 40, 42
    for i, (a, b) in enumerate(top):
        x = x0 + i * (w + gap)
        c.box(x, 48, w, h, a, b, stroke=t["warn"] if i == 1 else None, size=19)
        if i < 3:
            c.arrow(x + w, 107, x + w + gap - 4, 107)
    last = x0 + 3 * (w + gap) + w / 2
    first = x0 + w / 2
    c.line(f"M{last} 166C{last} 236 {last - 80} 224 {(first + last) / 2} 224C{first + 80} 224 {first} 212 {first} 278", t["mute"], 1.5)
    c.head(first, 282, math.pi / 2)
    for i, (a, b) in enumerate(low):
        x = x0 + i * (w + gap)
        final = i == 3
        c.box(x, 286, w, h, a, b, stroke=t["acc"] if final else (t["series"][1] if i == 0 else None), size=19)
        if i < 3:
            c.arrow(x + w, 345, x + w + gap - 4, 345)
    c.text("recusa aqui: nada é gravado", x0 + w + gap + w / 2, 200, 15, "mono", t["warn"], "middle")
    c.save()


def crates(t):
    c = Canvas("crates", 1280, 500, t, "Crates do workspace", "O app usa gfx, ed, graph e o núcleo; a linha de comando usa o núcleo.")
    c.box(250, 44, 340, 96, "app", "telas, grafo e janela", size=21, center=True, stroke=t["acc"])
    c.box(740, 44, 340, 96, "omnimind-cli", "MCP, hooks e CLI", size=21, center=True)
    low = [("gfx", "wgpu, cena e glifos", 42), ("ed", "texto e Markdown", 342), ("graph", "geometria pura", 642), ("omnimind-core", "cofre, diário, busca", 942)]
    for name, sub, x in low:
        c.box(x, 244, 296, 96, name, sub, size=20, center=True, stroke=t["acc"] if name == "omnimind-core" else None)
    for k, (_, _, x) in enumerate(low):
        c.curve(420 + (k - 1.5) * 10, 140, x + 148, 240, t["sub"], 1.5, horizontal=False)
    c.curve(910, 140, 1090, 240, t["sub"], 1.5, horizontal=False)
    deps = [("wgpu 30 · cosmic-text", 190), ("ropey", 490), ("bytemuck", 790), ("sem tokio · sem SQLite", 1090)]
    for label, x in deps:
        c.line(f"M{x} 340L{x} 392", t["line"], 1.5, "2 7")
        c.text(label, x, 420, 15.5, "mono", t["sub"], "middle")
    c.save()


# As mesmas curvas de `crates/app/src/bin/make_icon.rs`, em 64 unidades.
FEIXE = [[(10, 12), (27, 32), (37, 32), (54, 12)], [(7, 23), (25, 32), (39, 32), (57, 23)], [(6, 32), (6, 32), (58, 32), (58, 32)],
         [(7, 41), (25, 32), (39, 32), (57, 41)], [(10, 52), (27, 32), (37, 32), (54, 52)]]


def logo(c, x, y, size, color):
    k = size / 64
    at = lambda p: f"{x + (32 - p[1]) * k + size / 2:.1f} {y + (p[0] - 32) * k + size / 2:.1f}"
    for a, b, d, e in FEIXE:
        c.line(f"M{at(a)}C{at(b)} {at(d)} {at(e)}", color, 3.8 * k)


class Roots:
    """Colonização de espaço, como em `crates/graph/src/roots.rs`, com semente fixa."""

    def __init__(self, cx, cy, rx, ry, groups, seed=11, points=560):
        r = random.Random(seed)
        self.x, self.y, self.parent, self.group = [], [], [], []
        targets = []
        for _ in range(points):
            a = r.uniform(0, math.tau)
            d = math.sqrt(r.uniform(0, 1))
            g = int((a / math.tau) * groups) % groups
            targets.append((cx + rx * d * math.cos(a), cy + ry * d * math.sin(a), g))
        for g in range(groups):
            a = (g + 0.5) / groups * math.tau
            self.node(cx + 10 * math.cos(a), cy + 10 * math.sin(a), -1, g)
        grown = set()
        for _ in range(220):
            pull = {}
            alive = []
            for tx, ty, g in targets:
                best, near = -1, 74.0
                for i in range(len(self.x)):
                    if self.group[i] == g:
                        d = math.hypot(tx - self.x[i], ty - self.y[i])
                        if d < near:
                            best, near = i, d
                if best >= 0 and near < 11:
                    continue
                alive.append((tx, ty, g))
                if best >= 0:
                    dx, dy, n = pull.get(best, (0.0, 0.0, 0))
                    pull[best] = (dx + (tx - self.x[best]) / near, dy + (ty - self.y[best]) / near, n + 1)
            targets = alive
            if not pull:
                break
            for i, (dx, dy, n) in pull.items():
                size = math.hypot(dx, dy) or 1
                # O mesmo passo repetido empilha galhos no mesmo lugar e engrossa a ponta à toa.
                step = (i, round(dx / size * 4), round(dy / size * 4))
                if step not in grown:
                    grown.add(step)
                    self.node(self.x[i] + dx / size * 8, self.y[i] + dy / size * 8, i, self.group[i])
        self.smooth()
        self.width = self.weights()

    def node(self, x, y, parent, group):
        self.x.append(x)
        self.y.append(y)
        self.parent.append(parent)
        self.group.append(group)

    def smooth(self):
        for _ in range(3):
            kids = {}
            for i, p in enumerate(self.parent):
                kids.setdefault(p, []).append(i)
            x, y = self.x[:], self.y[:]
            for i, p in enumerate(self.parent):
                near = ([p] if p >= 0 else []) + kids.get(i, [])
                if p >= 0 and len(kids.get(i, [])) == 1:
                    x[i] = self.x[i] * 0.5 + sum(self.x[n] for n in near) / len(near) * 0.5
                    y[i] = self.y[i] * 0.5 + sum(self.y[n] for n in near) / len(near) * 0.5
            self.x, self.y = x, y

    def weights(self):
        leaves = [0] * len(self.x)
        has_kid = {p for p in self.parent if p >= 0}
        self.tips = [i for i in range(len(self.x)) if i not in has_kid]
        for i in self.tips:
            leaves[i] = 1
        for i in range(len(self.x) - 1, -1, -1):
            if self.parent[i] >= 0:
                leaves[self.parent[i]] += leaves[i]
        return [min(6.5, 0.8 + 0.72 * math.sqrt(n)) for n in leaves]

    def path_to(self, tip):
        points = []
        while tip >= 0:
            points.append((self.x[tip], self.y[tip]))
            tip = self.parent[tip]
        return points[::-1]


def banner(t):
    w, h = 1280, 440
    c = Canvas("banner", w, h, t, "Omnimind: a memória das suas IAs de código, em arquivos seus", "Logo, nome e frase, ao lado de um grafo em forma de raízes que cresce do centro.")
    tree = Roots(944, 226, 330, 300, len(t["series"]))
    c.items.append(f'<clipPath id="b"><rect x="1" y="1" width="{w - 2}" height="{h - 2}" rx="11"/></clipPath><g clip-path="url(#b)">')
    strokes = {}
    for i, p in enumerate(tree.parent):
        if p >= 0:
            key = (tree.group[i], round(tree.width[i] * 2) / 2)
            strokes.setdefault(key, []).append(f"M{tree.x[p]:.1f} {tree.y[p]:.1f}L{tree.x[i]:.1f} {tree.y[i]:.1f}")
    for (group, width), parts in sorted(strokes.items(), key=lambda kv: kv[0][1]):
        c.line("".join(parts), t["series"][group], width)
    for n, i in enumerate(tree.tips):
        c.dot(round(tree.x[i], 1), round(tree.y[i], 1), 2.2 + (n % 3) * 0.7, t["series"][tree.group[i]])
    c.dot(944, 226, 7, t["ink"])

    # Duas leituras percorrem o grafo; sem animação, os caminhos ficam acesos.
    far = sorted(tree.tips, key=lambda i: -math.hypot(tree.x[i] - 944, tree.y[i] - 226))
    reads = [far[0], next(i for i in far if tree.group[i] not in (tree.group[far[0]], (tree.group[far[0]] + 1) % 7, (tree.group[far[0]] - 1) % 7))]
    rules = ""
    for n, tip in enumerate(reads):
        points = tree.path_to(tip)
        length = sum(math.dist(a, b) for a, b in zip(points, points[1:]))
        d = "M" + "L".join(f"{x:.1f} {y:.1f}" for x, y in points)
        L = round(length) + 2
        c.items.append(f'<path class="read r{n}" d="{d}" fill="none" stroke="{t["ink"]}" stroke-width="2.4" stroke-linecap="round" stroke-dasharray="{L}"/>')
        c.items.append(f'<circle class="tip r{n}" cx="{points[-1][0]:.1f}" cy="{points[-1][1]:.1f}" r="7" fill="none" stroke="{t["ink"]}" stroke-width="1.6"/>')
        rules += (f"@keyframes read{n}{{0%{{stroke-dashoffset:{L};opacity:1}}42%{{stroke-dashoffset:0;opacity:1}}72%{{stroke-dashoffset:0;opacity:1}}86%,100%{{stroke-dashoffset:0;opacity:0}}}}"
                  f".read.r{n}{{animation:read{n} 9s ease-in-out {n * 3}s infinite both}}")
    rules += ("@keyframes tip{0%,40%{opacity:0}46%,72%{opacity:1}86%,100%{opacity:0}}.tip{animation:tip 9s linear infinite both}.tip.r1{animation-delay:3s}"
              "@media (prefers-reduced-motion:reduce){.read.r0,.read.r1,.tip,.tip.r1{animation:none}}")
    c.style = f"<style>{rules}</style>"
    c.items.append("</g>")

    logo(c, 56, 104, 92, t["ink"])
    c.text("Omnimind", 160, 180, 76, "strong")
    c.text("A memória das suas IAs de código,\nem arquivos seus.", 62, 256, 31, "sans")
    c.text("Corrija uma vez. Na próxima sessão, a IA já sabe.", 62, 358, 20, "sans", t["sub"])
    c.save()


THEMES = tokens()
DIAGRAMS = [banner, first_session, session, correction, projects, on_off, review, dream, architecture, write_path, crates]


def main():
    os.makedirs(OUT, exist_ok=True)
    for theme in THEMES.values():
        for draw in DIAGRAMS:
            draw(theme)
    print(len(DIAGRAMS) * 2, "imagens em", os.path.relpath(OUT, ROOT))


if __name__ == "__main__":
    main()
