#!/usr/bin/env python3
"""Simula uma IA lendo o cofre, para ver o grafo acender de verdade.

Sobe o servidor MCP real e chama `memory_read` em sequência: memória, nota, memória.
Cada leitura entra no diário do cofre, então rode numa CÓPIA, nunca no cofre que você usa.

  cp -R ~/Omnimind /tmp/cofre-de-teste
  target/release/omnimind-app --brain /tmp/cofre-de-teste &
  python3 scripts/simulate-reads.py --brain /tmp/cofre-de-teste
"""
import argparse
import glob
import json
import os
import random
import re
import subprocess
import sys
import time


def memory_ids(brain):
    ids = []
    for path in sorted(glob.glob(os.path.join(brain, "memories", "*", "*.md"))):
        with open(path, encoding="utf-8") as f:
            head = f.read(600)
        found = re.search(r"^id: (\S+)", head, re.M)
        # Arquivada não aparece no grafo: ler não acenderia nada.
        if found and not re.search(r"^archived:", head, re.M):
            ids.append(found.group(1))
    return ids


def note_paths(brain):
    notes = glob.glob(os.path.join(brain, "notes", "**", "*.md"), recursive=True)
    return sorted(os.path.relpath(p, brain) for p in notes)


class Server:
    def __init__(self, binary, brain, cwd):
        args = [binary, "mcp", "--brain", brain, "--client", "claude-code"]
        if cwd:
            args += ["--cwd", cwd]
        self.proc = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        self.next = 1

    def request(self, method, params):
        message = {"jsonrpc": "2.0", "id": self.next, "method": method, "params": params}
        self.next += 1
        self.proc.stdin.write(json.dumps(message) + "\n")
        self.proc.stdin.flush()
        return json.loads(self.proc.stdout.readline())

    def notify(self, method):
        self.proc.stdin.write(json.dumps({"jsonrpc": "2.0", "method": method}) + "\n")
        self.proc.stdin.flush()

    def read(self, target):
        answer = self.request("tools/call", {"name": "memory_read", "arguments": {"id": target, "max_chars": 400}})
        return answer.get("result", {}).get("structuredContent", {})

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--brain", required=True, help="pasta do cofre, de preferência uma cópia")
    parser.add_argument("--bin", default="target/release/omnimind", help="binário omnimind")
    parser.add_argument("--count", type=int, default=12, help="quantas leituras")
    parser.add_argument("--pause", type=float, default=2.2, help="segundos entre uma leitura e a próxima")
    parser.add_argument("--cwd", default="", help="pasta de projeto que a sessão simulada diz estar")
    parser.add_argument("--seed", type=int, default=7)
    args = parser.parse_args()

    brain = os.path.abspath(os.path.expanduser(args.brain))
    if not os.path.isfile(os.path.join(brain, "brain.toml")):
        sys.exit(f"{brain} não é um cofre: falta brain.toml")
    memories, notes = memory_ids(brain), note_paths(brain)
    if not memories:
        sys.exit("o cofre não tem memórias para ler")
    rng = random.Random(args.seed)
    rng.shuffle(memories)
    rng.shuffle(notes)

    server = Server(args.bin, brain, args.cwd)
    server.request("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "simulate-reads", "version": "0"}})
    server.notify("notifications/initialized")
    lit = 0
    for k in range(args.count):
        # A cada três, uma nota: é o que uma destilação faz.
        target = notes[(k // 3) % len(notes)] if notes and k % 3 == 1 else memories[k % len(memories)]
        got = server.read(target)
        status = got.get("status", "?")
        lit += status == "ok"
        print(f"{k + 1:>2}/{args.count}  {status:<8} {got.get('title') or target}", flush=True)
        time.sleep(args.pause)
    server.close()
    print(f"{lit} de {args.count} leituras responderam ok.")


if __name__ == "__main__":
    main()
