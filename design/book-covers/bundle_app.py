import urllib.request,xml.etree.ElementTree as E,json,pathlib,hashlib,shutil
r=pathlib.Path(__file__).resolve().parents[2];bs=json.loads((r/'design/book-covers/collection/books.json').read_text())['books'];out=[]
for v,a,z in [(1,1,5),(2,6,17),(3,18,22),(4,23,39),(5,40,43),(6,44,66)]:
 path=pathlib.Path(f'/tmp/mhc{v}.xml');url=f'https://www.ccel.org/ccel/h/henry/mhc{v}.xml'
 if not path.exists():path.write_bytes(urllib.request.urlopen(url).read())
 tree=E.parse(path); sections=[d for d in tree.iter('div1') if any(c.attrib.get('title')=='Introduction' for c in d.findall('div2'))];assert len(sections)==z-a+1,(v,len(sections))
 for b,d in zip(bs[a-1:z],sections):
  intro=next(c for c in d.findall('div2') if c.attrib.get('title')=='Introduction');ps=[];credit=[]
  for p in intro.findall('p'):
   text=' '.join(''.join(p.itertext()).split())
   if text.startswith('Completed by'):credit.append(text)
   elif len(text)>100:ps.append(text)
  assert ps,b['name']
  if b['id']==1:ps=[ps[-1]]
  if b['id']==40:ps=[ps[-1]]
  art=(r/'design/book-covers/collection'/b['art']).resolve();target=r/f"assets/ui/book-covers/{b['osis']}.svg";shutil.copyfile(art,target)
  out.append(dict(id=b['id'],name=b['name'],title=b['title'],paragraphs=ps,attribution='Matthew Henry’s Commentary'+(' · Henry and continuators' if b['id']>=44 else ''),credit=' '.join(credit),source=f"https://www.ccel.org/ccel/henry/mhc{v}.{intro.attrib['id']}.html",source_xml=url,source_sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
(r/'assets/ui/book-covers/introductions.json').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
(r/'crates/ui-egui/src/book_art.rs').write_text('pub(super) static ART: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("../../../assets/ui/book-covers/{b["osis"]}.svg"),\n' for b in bs)+'];\n')
for b in out:print(b['id'],b['name'],len(b['paragraphs']),sum(map(len,b['paragraphs'])))
