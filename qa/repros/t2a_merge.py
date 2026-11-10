import sys; sys.path.insert(0,'.')
from vz import *
pairs=[
 ("Deploy da rota A","Deploy da rota A é às 10h","Deploy da rota B","Deploy da rota B é às 11h"),
 ("Deploy rota","Deploy da rota A é às 10h","Deploy rota","Deploy da rota B é às 11h"),
 ("Deploy da rota A","Deploy da rota A é às 10h","Deploy da rota A","Deploy da rota A é às 11h"),
 ("Horário do deploy da rota A em produção","O deploy da rota A em produção acontece às 10h da manhã de terça","Horário do deploy da rota B em produção","O deploy da rota B em produção acontece às 10h da manhã de terça"),
 ("Deploy produção rota A","O deploy da rota A em produção acontece às 10h da manhã de terça-feira, conforme combinado com o time de plataforma e registrado na reunião de outubro","Deploy produção rota B","O deploy da rota B em produção acontece às 10h da manhã de terça-feira, conforme combinado com o time de plataforma e registrado na reunião de outubro"),
 ("Servidor de produção","O servidor de produção fica em São Paulo","Servidor de homologação","O servidor de homologação fica em São Paulo"),
 ("Limite da API de pagamentos","O limite da API de pagamentos é 100 req/s","Limite da API de pedidos","O limite da API de pedidos é 100 req/s"),
 ("Limite da API","O limite da API é 100 req/s","Limite da API","O limite da API é 1000 req/s"),
 ("Fulano mora no Rio","Fulano mora no Rio","Fulano não mora no Rio","Fulano não mora no Rio"),
 ("Cache habilitado","O cache está habilitado em produção","Cache desabilitado","O cache está desabilitado em produção"),
 ("Usar Postgres na rota","Usar Postgres na rota certa em 2026","Usar MySQL na rota","Usar MySQL na rota certa em 2026"),
 ("Reunião com Ana","Reunião com Ana na terça às 14h na sala azul do segundo andar","Reunião com Bia","Reunião com Bia na terça às 14h na sala azul do segundo andar"),
 ("Credencial do ambiente X","Credencial do ambiente X guardada no cofre A na pasta de plataforma do time","Credencial do ambiente Y","Credencial do ambiente Y guardada no cofre B na pasta de plataforma do time"),
 ("Prazo do projeto Alfa","Prazo do projeto Alfa é dia 10 de dezembro de 2026, sem exceção","Prazo do projeto Beta","Prazo do projeto Beta é dia 20 de dezembro de 2026, sem exceção"),
 ("Versão do Node no serviço web","Serviço web roda Node 20 em produção desde março de 2026","Versão do Node no serviço api","Serviço api roda Node 22 em produção desde março de 2026"),
 ("Regra de desconto","Desconto de 10% para clientes ouro aprovado pela diretoria em setembro de 2026","Regra de desconto","Desconto de 15% para clientes ouro aprovado pela diretoria em setembro de 2026"),
 ("Regra de desconto clientes prata","Desconto de 10% para clientes prata aprovado pela diretoria em setembro de 2026","Regra de desconto clientes ouro","Desconto de 10% para clientes ouro aprovado pela diretoria em setembro de 2026"),
 ("Aniversário da Maria","Aniversário da Maria é 12 de março","Aniversário do João","Aniversário do João é 12 de março"),
 ("Telefone do suporte","Telefone do suporte: 11 4002-8922","Telefone do suporte","Telefone do suporte: 11 4002-8923"),
 ("Porta do serviço","O serviço escuta na porta 8080 em todos os ambientes","Porta do serviço","O serviço escuta na porta 9090 em todos os ambientes"),
 ("Decisão sobre cache","Não usar Redis para sessão porque o custo é alto","Decisão sobre cache","Usar Redis para sessão porque o custo é baixo"),
 ("Time responsável","Rota certa pertence ao time Azul","Time responsável","Rota certa pertence ao time Verde"),
]
b=None
tot=dict(lost=0,kept=0)
for i,(t1,x1,t2,x2) in enumerate(pairs):
  for field in ('summary','body'):
    for typ in ('decision',):
      b=fresh('t2a')
      m=Mcp(b)
      k1={field:x1}; k2={field:x2}
      a=m.write(type=typ,title=t1,origin_kind='session',reason='qa',**k1)
      c=m.write(type=typ,title=t2,origin_kind='session',reason='qa',**k2)
      txt=[p.read_text(encoding='utf8') for p in (b/'memories').rglob('*.md')]
      nmem=len(txt)
      lost = not any(x2 in t for t in txt)
      s=m.search(x2)
      found=any(x2[:40] in json.dumps(s['results'],ensure_ascii=False) for _ in [0])
      tot['lost' if lost else 'kept']+=1
      print(f"{'LOST' if lost else 'ok  '} {field:7} {c.get('outcome'):9} rule={c.get('rule'):16} warn={str(c.get('warnings'))[:40]:40} files={nmem} | {x2[:55]}")
      m.close()
print(tot)
