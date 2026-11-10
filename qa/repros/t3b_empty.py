import sys,time,os,threading,collections,statistics; sys.path.insert(0,'.')
from vz import *
print('[hook latency with 76k-event journal]')
b=QA/'t4'
ts=[]
for i in range(20):
    t=time.time(); r=hook('session-start',b,{'session_id':f'lat{i}','cwd':str(b)}); ts.append(time.time()-t)
print('session-start ms p50',round(statistics.median(ts)*1000,1),'max',round(max(ts)*1000,1),'bytes',len(r.stdout))
ts=[]
for i in range(20):
    t=time.time(); hook('prompt',b,{'session_id':f'lat{i}','cwd':str(b),'prompt':'não, prefiro isso'}); ts.append(time.time()-t)
print('prompt ms p50',round(statistics.median(ts)*1000,1),'max',round(max(ts)*1000,1))
print(subprocess.run(f"du -sh {b}/.omnimind/journal; ls {b}/.omnimind/journal/*/ | wc -l",shell=True,capture_output=True,text=True).stdout)
t=time.time(); x=Mcp(b); r=x.search('ancora'); print('open+first search on 76k journal s',round(time.time()-t,3),r['status']); x.close()

print('[phantom unread after delete]')
b=fresh('t3p'); (b/'notes').mkdir(exist_ok=True)
for i in range(20): (b/f'notes/n{i}.md').write_text(f'# n{i}\nmarcador{i}x\n')
m=Mcp(b); m.search('marcador1x'); (b/'notes/n3.md').unlink(); t0=time.time(); seq=[]
for i in range(16):
    r=m.search('marcador4x'); seq.append((round(time.time()-t0,2),r['status'],r['coverage']['unread'],r['coverage']['notes'])); time.sleep(.1)
print(seq); 
m.close()
b=fresh('t3p2'); (b/'notes').mkdir(exist_ok=True)
for i in range(20): (b/f'notes/n{i}.md').write_text(f'# n{i}\nmarcador{i}x\n')
m=Mcp(b); m.search('marcador1x'); (b/'notes/n3.md').unlink(); r=m.search('marcador3x'); print(r['status'],r['coverage']['unread']); r=m.read('notes/n3.md'); print('read',r.get('status')); seq=[];t0=time.time()
for i in range(16):
    r=m.search('marcador4x'); seq.append((round(time.time()-t0,2),r['status'],r['coverage']['unread'],r['coverage']['notes'])); time.sleep(.1)
print(seq); m.close()

print('[non-atomic editor: truncate, 30 ms, write] reader for the same term')
b=fresh('t3t'); (b/'notes').mkdir(exist_ok=True)
for i in range(50): (b/f'notes/n{i}.md').write_text(f'# n{i}\nmarcador{i}x\n')
stop=time.time()+8; p=b/'notes/n7.md'; body=p.read_text()
def ed():
    while time.time()<stop:
        with open(p,'w') as f:
            f.flush(); time.sleep(.03); f.write(body)
        time.sleep(.02)
th=threading.Thread(target=ed); th.start()
m=Mcp(b); c=collections.Counter()
while time.time()<stop:
    r=m.search('marcador7x'); c[(r['status'],len(r['results'])>0,r['coverage']['unread'])]+=1
th.join(); m.close(); print(c)

print('[agent memory rewritten via CAS: reader during rewrite]')
b=fresh('t3c'); m=Mcp(b,session='a'); a=m.write(type='concept',title='Alvo reescrita',summary='termofixo ok',body='x',origin_kind='session'); 
stop=time.time()+6
def wr():
    h=a['hash']; i=0
    while time.time()<stop:
        i+=1; r=m.write(id=a['id'],base_hash=h,title='Alvo reescrita',summary=f'termofixo versao {i}',body=f'x{i}',reason='q'); h=r.get('hash',h)
th=threading.Thread(target=wr); th.start(); r2=Mcp(b,session='b'); c=collections.Counter()
while time.time()<stop:
    r=r2.search('termofixo'); c[(r['status'],len(r['results'])>0)]+=1
th.join(); print(c); m.close(); r2.close()

print('[cold start with big brain, no snapshot]')
b=fresh('t3big'); (b/'notes').mkdir(exist_ok=True)
N=30000
for i in range(N): (b/f'notes/n{i:05d}.md').write_text(f'# n{i}\nmarcador{i}x texto\n'+('palavra%d '%(i%997))*60)
x=Mcp(b); t=time.time(); r=x.search(f'marcador{N-1}x'); print(round(time.time()-t,2),r['status'],r.get('reason'),len(r['results']),r['coverage'])
t=time.time(); r=x.search('naoexistemesmoqwzk'); print(round(time.time()-t,2),r['status'],r.get('reason'),r['coverage']); 
for _ in range(5):
    time.sleep(.5); t=time.time(); r=x.search(f'marcador{N-1}x'); print(round(time.time()-t,2),r['status'],len(r['results']),r['coverage'])
x.close()
print(os.path.getsize(b/'.omnimind/snapshot.bin') if (b/'.omnimind/snapshot.bin').exists() else 'nosnap')
subprocess.run(['rm','-rf',str(b)])
