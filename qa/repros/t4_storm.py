import sys,time,os,threading,random,signal,collections; sys.path.insert(0,'.')
from vz import *
b=fresh('t4'); (b/'notes').mkdir(exist_ok=True)
for i in range(200):
    (b/f'notes/n{i:03d}.md').write_text(f"# Nota {i}\nTexto {i} marcador{i}x logistica comum.\n")
s=Mcp(b,session='seed')
st=s.write(type='decision',title='Ancora estavel',summary='ancora estavelzzz nunca muda',body='corpo fixo',origin_kind='session')
cas=s.write(type='concept',title='Alvo do CAS',summary='alvo cas versao zero',body='zero',origin_kind='session')
s.close()
DUR=25; stop=time.time()+DUR
res=collections.defaultdict(list); lock=threading.Lock()
def rec(k,v):
    with lock: res[k].append(v)
def writer(n):
    m=Mcp(b,session=f'w{n}',client='claude-code')
    i=0
    while time.time()<stop and i<38:
        i+=1
        tok=f"wz{n}q{i}k"
        t0=time.time()
        r=m.write(type='episode',title=f'Evento unico {tok}',summary=f'{tok} em {random.random()} sobre assunto {n*1000+i}',body=f'fato {tok} '+' '.join(random.sample(['alfa','beta','gama','delta','epsilon','zeta','eta','teta','iota','kapa'],6)),origin_kind='session',key=tok)
        rec('wlat',time.time()-t0)
        if r.get('outcome')=='written': rec('written',(tok,r['id']))
        else: rec('wother',(tok,r.get('outcome'),r.get('reason'),r.get('error'),str(r.get('message'))[:100]))
        if i%3==0:
            s=m.search(tok)
            if not any(tok in x['title'] for x in s['results']): rec('ryow_miss',(tok,s['status'],s['coverage']))
    # repeat same key from a second process: idempotent
    rec('rc',m.close())
def casw(n):
    m=Mcp(b,session=f'c{n}',client='copilot')
    cur=m.read(cas['id'])
    while time.time()<stop:
        h=cur.get('hash')
        r=m.write(id=cas['id'],base_hash=h,title='Alvo do CAS',summary=f'alvo cas de {n} {random.random()}',body=f'por {n}',reason='qa')
        rec('cas',r.get('outcome')+':'+str(r.get('reason')))
        if r.get('outcome') in('written',): rec('caswin',(n,r.get('hash')))
        cur=m.read(cas['id'])
        time.sleep(random.random()*.05)
    rec('rc',m.close())
def reader(n):
    m=Mcp(b,session=f'r{n}',client='qa-reader')
    c=0
    while time.time()<stop:
        c+=1
        r=m.search('ancora estavelzzz')
        ok=any(x['id']==st['id'] for x in r.get('results',[]))
        if not ok: rec('anchor_miss',(r.get('status'),r.get('coverage'),str(r.get('error'))))
        i=random.randrange(40,200)
        r2=m.search(f'marcador{i}x')
        if not any(f'n{i:03d}' in x['id'] for x in r2.get('results',[])): rec('note_miss',(i,r2.get('status'),r2.get('reason'),r2.get('coverage')))
        rec('rstatus',r.get('status'))
    rec('rcount',c)
    rec('rc',m.close())
def hooks(n):
    i=0
    while time.time()<stop:
        i+=1
        for ev,p in [('session-start',{'source':'startup'}),('prompt',{'prompt':f'não, prefiro abordagem {n}-{i} aqui'}),('post-tool',{'tool_name':'Read','tool_input':{'file_path':'/x/y.md'}}),('stop',{'last_assistant_message':f'uso [mem:{st["id"]}]'}),('pre-compact',{}),('session-end',{})]:
            r=hook(ev,b,{'session_id':f'hk{n}-{i}','cwd':str(b),**p})
            if r.returncode!=0 or 'panic' in r.stderr: rec('hookfail',(ev,r.returncode,r.stderr[:200]))
            rec('hooks',1)
def editor():
    # editor-like: atomic rename of notes 0..39, plus truncate-and-write on 20..39 (non-atomic)
    i=0
    while time.time()<stop:
        i+=1
        k=random.randrange(40); p=b/f'notes/n{k:03d}.md'
        t=p.read_text()+f'\nlinha {i}\n'
        q=b/f'notes/.tmp{k}'; q.write_text(t); os.replace(q,p)
        time.sleep(.01)
def sleeper():
    while time.time()<stop:
        r=cli('sleep',brain=b)
        rec('sleep',r.returncode)
        if r.returncode or 'panic' in r.stderr: rec('sleepfail',r.stderr[:300])
        time.sleep(.5)
def killer():
    # spawn MCP writers and kill -9 them mid-write
    n=0
    while time.time()<stop:
        n+=1
        m=Mcp(b,session=f'k{n}')
        def w():
            try:
                for i in range(30):
                    tok=f'kz{n}q{i}k'; r=m.write(type='episode',title=f'Morto {tok}',summary=f'{tok} s',body=f'c {tok}',origin_kind='session',key=tok)
                    if r.get('outcome')=='written': rec('killed_written',(tok,r['id']))
            except Exception: pass
        th=threading.Thread(target=w); th.start(); time.sleep(random.random()*.4); m.p.send_signal(signal.SIGKILL); th.join(); rec('kills',1)
T=[threading.Thread(target=writer,args=(i,)) for i in range(8)]+[threading.Thread(target=casw,args=(i,)) for i in range(2)]+[threading.Thread(target=reader,args=(i,)) for i in range(6)]+[threading.Thread(target=hooks,args=(i,)) for i in range(4)]+[threading.Thread(target=editor),threading.Thread(target=sleeper),threading.Thread(target=killer)]
[t.start() for t in T]; [t.join() for t in T]
print({k:len(v) for k,v in res.items()})
print('w lat p50/p95/max',sorted(res['wlat'])[len(res['wlat'])//2],sorted(res['wlat'])[int(len(res['wlat'])*.95)],max(res['wlat']))
print('rc',collections.Counter(str(x) for x in res['rc']).most_common(6))
print('wother',collections.Counter((a[1],a[2],a[3]) for a in res['wother']).most_common(8)); print(res['wother'][:2])
print('cas',collections.Counter(res['cas']))
print('anchor_miss',len(res['anchor_miss']),collections.Counter(str(x[0]) for x in res['anchor_miss']),res['anchor_miss'][:3])
print('note_miss',len(res['note_miss']),collections.Counter(str(x[1]) for x in res['note_miss']),res['note_miss'][:3])
print('ryow_miss',len(res['ryow_miss']),res['ryow_miss'][:3])
print('rstatus',collections.Counter(res['rstatus']))
print('hookfail',res['hookfail'][:3],'sleepfail',res['sleepfail'][:3], 'sleeps', collections.Counter(res['sleep']))
# verification
m=Mcp(b,session='check'); time.sleep(.5)
files={p.read_text().split('\n')[1].split(': ')[1]:p for p in (b/'memories').rglob('*.md')}
lost=[(t,i) for t,i in res['written']+res['killed_written'] if i not in files]
print('written',len(res['written']),'killed_written',len(res['killed_written']),'lost files',len(lost),lost[:3])
ns=0
for t,i in res['written']:
    r=m.search(t)
    if not any(x['id']==i for x in r['results']):
        ns+=1; print(' notfound',t,r['status'])
print('not searchable',ns)
m.close()
print(cli('doctor',brain=b).stdout)
print(cli('lint',brain=b).stdout[-400:])
import glob
ev=collections.Counter(); bad=0; seen=set(); dup=0
for f in glob.glob(str(b/'.omnimind/journal/*/*.jsonl')):
    for l in open(f,encoding='utf8'):
        try:
            e=json.loads(l); ev[e['ev']]+=1
            if e['id'] in seen: dup+=1
            seen.add(e['id'])
        except Exception: bad+=1
print(ev,'bad',bad,'dup',dup)
w_ids={e for e in res['written']}
print('tmp left',list((b/'.omnimind/tmp').glob('*'))[:5] if (b/'.omnimind/tmp').exists() else None)
sig=[e for e in ev if e=='signal']; print('signals expected', len(res['hooks'])//6,'journal',ev['signal'])
