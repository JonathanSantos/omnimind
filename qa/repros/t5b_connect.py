import sys,time,os,hashlib; sys.path.insert(0,'.')
from vz import *
H=QA/'t5home'; subprocess.run(['rm','-rf',str(H)]); H.mkdir()
(H/'.claude').mkdir(); (H/'.claude/settings.json').write_text('{"keep":"home"}\n'); (H/'.claude.json').write_text('{"home":1}\n')
Bx=fresh('t5conn')
def tree(p):
    o={}
    for r,ds,fs in os.walk(p):
        for f in fs:
            q=pathlib.Path(r)/f
            if q.is_symlink(): o[str(q.relative_to(p))]='L'; continue
            o[str(q.relative_to(p))]=hashlib.sha1(q.read_bytes()).hexdigest()[:8]
    return o
def con(*a,cwd=None,brain=Bx,home=H):
    args=[VZ,'connect',*map(str,a)]
    if brain: args+=['--brain',str(brain)]
    return subprocess.run(args,capture_output=True,text=True,env=env(home),cwd=cwd or str(QA),timeout=20)
P=QA/'t5proj'; subprocess.run(['rm','-rf',str(P)]); P.mkdir()
(P/'.mcp.json').write_text('{\n  "mcpServers": {"outro": {"command": "x"}},\n  "other": 1\n}\n')
(P/'.claude').mkdir(); (P/'.claude/settings.json').write_text('{"permissions":{"allow":["Bash(ls)"]},"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"echo meu"}]}]}}')
h0=tree(H); b0=tree(Bx)
r=con('claude-code','--project',P,'--print'); print('PRINT rc',r.returncode,'\n',r.stdout[:1500],r.stderr[:300]); print('print wrote?',tree(P).keys())
r=con('claude-code','--project',P); print('REAL rc',r.returncode,r.stdout[:900],r.stderr[:300])
t1=tree(P); print(sorted(t1))
print(open(P/'.mcp.json').read()); print(open(P/'.claude/settings.json').read()[:2500])
r2=con('claude-code','--project',P); print('SECOND rc',r2.returncode,r2.stdout[:400],r2.stderr[:300]); t2=tree(P); print('changed on 2nd run:',{k for k in set(t1)|set(t2) if t1.get(k)!=t2.get(k)})
print('HOME changed:',tree(H)!=h0,' brain changed:',tree(Bx)!=b0)
bk=sorted(k for k in t2 if 'omnimind-backup' in k); print('backups',bk); 
for k in bk: print(' ',k,(P/k).read_text()[:80].replace('\n',' '))
print('\n-- run the hook command as written')
st=json.load(open(P/'.claude/settings.json'))
cmds=[h['command'] for e,v in st['hooks'].items() for g in v for h in g['hooks']]; print(cmds[:7])
c=[x for x in cmds if 'session-start' in x][0]
p=subprocess.run(['sh','-c',c],input='{"session_id":"cn"}',capture_output=True,text=True,env=env(H),cwd=P); print(p.returncode,p.stdout[:160],p.stderr[:200])
print('\n-- user-edited skill then reconnect')
sk=P/'.claude/skills/omnimind/SKILL.md'; sk.write_text(sk.read_text()+'\nMINHA EDICAO\n'); r=con('claude-code','--project',P); print(r.returncode,r.stdout[:300]); print('kept edit:','MINHA EDICAO' in sk.read_text(),[k for k in tree(P) if 'SKILL' in k and 'backup' in k])
print('\n-- invalid JSON untouched')
Q=QA/'t5proj2'; subprocess.run(['rm','-rf',str(Q)]); Q.mkdir(); (Q/'.mcp.json').write_text('{ invalido'); r=con('claude-code','--project',Q); print(r.returncode,r.stdout[:200],r.stderr[:300],(Q/'.mcp.json').read_text(),sorted(tree(Q)))
for name,c_ in [('array','[]'),('servers-array','{"mcpServers":[]}'),('hooks-str',None)]:
    Q=QA/'t5proj3'; subprocess.run(['rm','-rf',str(Q)]); Q.mkdir()
    if c_: (Q/'.mcp.json').write_text(c_)
    else: (Q/'.claude').mkdir(); (Q/'.claude/settings.json').write_text('{"hooks":"x"}')
    r=con('claude-code','--project',Q); print(name,r.returncode,r.stdout[:120].replace('\n',' '),r.stderr[:200], sorted(tree(Q))[:3], ((Q/'.mcp.json').read_text()[:100] if (Q/'.mcp.json').exists() else ''))
print('\n-- JSONC in .vscode/mcp.json');Q=QA/'t5proj4'; subprocess.run(['rm','-rf',str(Q)]); (Q/'.vscode').mkdir(parents=True); (Q/'.vscode/mcp.json').write_text('{\n // comentario\n "servers": {},\n}\n'); r=con('copilot','--project',Q); print(r.returncode,r.stdout[:200],r.stderr[:300]); print(sorted(tree(Q)))
print('\n-- symlinks')
Q=QA/'t5proj5'; subprocess.run(['rm','-rf',str(Q)]); Q.mkdir(); (Q/'.claude').symlink_to(H/'.claude'); r=con('claude-code','--project',Q); print('.claude->HOME',r.returncode,r.stdout[:150],r.stderr[:200]); print('home intact',tree(H)==h0,sorted(tree(Q)))
Q=QA/'t5proj6'; subprocess.run(['rm','-rf',str(Q)]); Q.mkdir(); (Q/'.mcp.json').symlink_to(H/'.claude.json'); r=con('claude-code','--project',Q); print('.mcp->HOME',r.returncode,r.stdout[:150],r.stderr[:200]); print('home intact',tree(H)==h0)
Q=QA/'t5proj7'; subprocess.run(['rm','-rf',str(Q)]); Q.mkdir(); (Q/'.claude').mkdir(); (Q/'.claude/skills').symlink_to(H/'.claude'); r=con('claude-code','--project',Q); print('skills->HOME',r.returncode,r.stderr[:200]); print('home intact',tree(H)==h0)
print('\n-- paths with spaces and shell metachars')
for nm in ["proj com espaco","proj'aspas","proj$(touch PWNED)x","proj;touch PWNED;x",'proj"dq',"projção"]:
    Q=QA/('t5q'/pathlib.Path(nm)) if False else QA/'t5q'/nm
    subprocess.run(['rm','-rf',str(QA/'t5q')]); Q.mkdir(parents=True)
    Bn=QA/'t5qb'/nm; subprocess.run(['rm','-rf',str(QA/'t5qb')]); Bn.parent.mkdir(parents=True); r0=cli('init',Bn)
    r=con('claude-code','--project',Q,brain=Bn); 
    st=json.load(open(Q/'.claude/settings.json')); c=[h['command'] for e,v in st['hooks'].items() for g in v for h in g['hooks'] if 'session-start' in h['command']][0]
    w=QA/'t5cwd'; subprocess.run(['rm','-rf',str(w)]); w.mkdir()
    p=subprocess.run(['sh','-c',c],input='{"session_id":"cn"}',capture_output=True,text=True,env=env(H),cwd=w)
    srv=json.load(open(Q/'.mcp.json'))['mcpServers']['omnimind']
    print(repr(nm),'connect',r.returncode,'hook rc',p.returncode,'output_ok','Mem' in p.stdout,'injected',bool(list(w.glob('PWNED'))+list(pathlib.Path('.').glob('PWNED'))),'journal',bool(list((Bn/'.omnimind/journal').rglob('*.jsonl'))) if (Bn/'.omnimind').exists() else None, p.stderr[:80])
print('\n-- copilot')
Q=QA/'t5proj8'; subprocess.run(['rm','-rf',str(Q)]); Q.mkdir(); r=con('copilot','--project',Q); print(r.returncode,r.stdout[:300]); t=tree(Q); print(sorted(t)); r=con('copilot','--project',Q); print('2nd changes',t!=tree(Q)); 
for f in ['.vscode/mcp.json','.github/mcp.json','.github/hooks/omnimind.json']: print(f,(Q/f).read_text()[:500])
print('\n-- global uses HOME only')
h1=tree(H); Q=QA/'t5proj9'; subprocess.run(['rm','-rf',str(Q)]); Q.mkdir(); r=con('claude-code','--global',cwd=Q); print(r.returncode,r.stdout[:400],r.stderr[:200]); h2=tree(H); print('changed:',sorted(k for k in set(h1)|set(h2) if h1.get(k)!=h2.get(k))); print('project untouched',tree(Q)=={})
print('\n-- errors')
print(con('foo','--project',P).stderr[:200]); print(con('claude-code','--project',QA/'naoexiste').stderr[:200],(QA/'naoexiste').exists())
print(con('claude-code','--project',P,brain=QA/'naoexistebrain').stderr[:200]); r=con('claude-code','--project',Q,brain=None); print('no brain:',r.returncode,r.stdout[:150],r.stderr[:200])
