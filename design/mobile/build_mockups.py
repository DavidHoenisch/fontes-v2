from pathlib import Path
import json, html, re, urllib.request
from PIL import ImageFont

ROOT = Path(__file__).parent
OUT = ROOT / 'screens'
OUT.mkdir(exist_ok=True)
ICONS = json.loads((ROOT/'assets/phosphor-regular.json').read_text())
for key, name in [('wifi','wifi-high'),('battery','battery-full'),('minus','minus')]:
    ICONS[key] = urllib.request.urlopen(f'https://raw.githubusercontent.com/phosphor-icons/core/main/assets/regular/{name}.svg').read().decode()
COL = dict(desk='#1C201E', sidebar='#151917', surface='#252A27', raised='#2C322E', border='#3C443E', ink='#ECEDE5', muted='#ABB3AA', quiet='#828E83', accent='#DCBC83', tint='#38372B', inverse='#1C211B', green='#B2C5AB')
FONTS = {'sans':('IBM Plex Sans','IBMPlexSans-Regular.ttf'), 'medium':('IBM Plex Sans','IBMPlexSans-Medium.ttf'), 'serif':('Lora','Lora-Regular.ttf'), 'greek':('Noto Serif','NotoSerif-Regular.ttf')}
def color(c): return COL.get(c,c)
def esc(s): return html.escape(str(s),quote=True)

class Screen:
    def __init__(self,name):
        self.name=name; self.parts=[]; self.audit=[]
        self.rect(0,0,390,844,'desk')
        self.status()
    def rect(self,x,y,w,h,fill='surface',r=0,stroke=None,opacity=1):
        self.parts.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{color(fill)}" opacity="{opacity}"'+(f' stroke="{color(stroke)}"' if stroke else '')+'/>')
    def line(self,x,y,w): self.rect(x,y,w,1,'border')
    def text(self,x,y,s,size=16,font='sans',fill='ink',width=None,line=None,anchor='start'):
        ff,fn=FONTS[font]; f=ImageFont.truetype(str(ROOT/'assets/fonts'/fn),size)
        lines=[]
        for paragraph in str(s).split('\n'):
            if width is None: lines.append(paragraph);continue
            acc=''
            for word in paragraph.split():
                proposed=(acc+' '+word).strip()
                if acc and f.getlength(proposed)>width: lines.append(acc);acc=word
                else: acc=proposed
            lines.append(acc)
        lh=line or (32 if font=='serif' and size==20 else size*1.4)
        self.parts.append(f'<text font-family="{ff}" font-size="{size}" font-weight="{500 if font=="medium" else 400}" fill="{color(fill)}" text-anchor="{anchor}">')
        for i,l in enumerate(lines):
            by=y+i*lh
            self.parts.append(f'<tspan x="{x}" y="{by}">{esc(l)}</tspan>')
            length=f.getlength(l);left=x-length/2 if anchor=='middle' else x
            self.audit.append(dict(text=l,x=round(left,1),right=round(left+length,1),baseline=by))
        self.parts.append('</text>')
        return len(lines)*lh
    def icon(self,x,y,key,fill='muted',size=24):
        source=ICONS[key];inside=re.sub(r'^.*?<svg[^>]*>|</svg>\s*$','',source,flags=re.S)
        self.parts.append(f'<g id="phosphor-{key}" transform="translate({x} {y}) scale({size/256})" fill="{color(fill)}">{inside}</g>')
    def icon_button(self,x,y,key,fill='muted',bg=None):
        if bg:self.rect(x,y,44,44,bg,10)
        self.icon(x+10,y+10,key,fill)
    def button(self,x,y,w,label,primary=False,key=None):
        self.rect(x,y,w,48,'accent' if primary else 'raised',10,None if primary else 'border')
        self.text(x+w/2-(10 if key else 0),y+30,label,16,'medium','inverse' if primary else 'ink',anchor='middle')
        if key:self.icon(x+w-36,y+12,key,'inverse' if primary else 'muted')
    def status(self):
        self.text(24,23,'9:41',12,'medium');self.icon(312,10,'wifi','muted',16);self.icon(340,9,'battery','muted',20)
    def header(self,title='John 3',back=False,extra=True):
        if back:self.icon_button(12,40,'back');self.text(64,68,title,20,'medium')
        else:
            self.icon(24,51,'book','accent');self.text(60,70,title,20,'medium');self.icon(126,54,'down','muted',20)
        if extra:
            self.rect(244,44,58,40,'raised',10);self.text(273,69,'KJV',12,'medium',anchor='middle');self.icon_button(318,42,'bookmark')
        self.line(0,91,390)
    def nav(self,active='Read'):
        self.rect(0,760,390,84,'sidebar');self.line(0,760,390)
        for i,(label,key) in enumerate([('Read','book'),('Search','search'),('Notes','notes'),('Study','tree')]):
            cx=48.75+i*97.5
            if label==active:self.rect(cx-28,768,56,32,'raised',12)
            self.icon(cx-12,772,key,'accent' if label==active else 'quiet',24)
            self.text(cx,818,label,12,'medium','accent' if label==active else 'muted',anchor='middle')
        self.rect(145,834,100,4,'quiet',2)
    def chip(self,x,y,w,label,selected=False):
        self.rect(x,y,w,36,'tint' if selected else 'raised',8)
        self.text(x+w/2,y+23,label,12,'medium','accent' if selected else 'muted',anchor='middle')
    def label(self,x,y,s):self.text(x,y,s,12,'medium','muted')
    def save(self):
        bad=[a for a in self.audit if a['x'] < -0.1 or a['right'] > 390.1 or a['baseline']>839]
        if bad:raise ValueError((self.name,bad))
        svg=f'<svg xmlns="http://www.w3.org/2000/svg" width="390" height="844" viewBox="0 0 390 844"><title>{esc(self.name)}</title>'+''.join(self.parts)+'</svg>'
        (OUT/(self.name+'.svg')).write_text(svg)
        return svg

screens=[]
s=Screen('01-reading');s.header();s.nav()
s.label(24,125,'THE GOSPEL ACCORDING TO JOHN')
s.text(24,167,'God so loved\nthe world',28,'serif',line=36)
s.label(24,234,'JOHN 3:14–16 · AVAILABLE OFFLINE')
y=278
for num,verse in [(14,'And as Moses lifted up the serpent in the wilderness, even so must the Son of man be lifted up:'),(15,'That whosoever believeth in him should not perish, but have eternal life.'),(16,'For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.')]:
    f=ImageFont.truetype(str(ROOT/'assets/fonts/Lora-Regular.ttf'),20)
    # Passages are continuous reading content, with generous leading.
    words=verse.split();lines=[];acc=''
    for word in words:
        if acc and f.getlength(acc+' '+word)>304:lines.append(acc);acc=word
        else:acc=(acc+' '+word).strip()
    lines.append(acc);h=len(lines)*28
    if num==16:s.rect(16,y-24,358,h+16,'tint',10)
    s.text(24,y-4,str(num),12,'sans','accent' if num==16 else 'quiet')
    s.text(54,y,verse,20,'serif',width=304,line=28)
    y+=h+24
screens.append(s)

s=Screen('02-select-book');s.header('Go to a passage',True,False);s.icon_button(334,42,'close')
s.rect(24,112,342,48,'raised',10,'border');s.icon(38,124,'search');s.text(74,142,'Reference or book name',16,'sans','muted')
s.rect(24,180,342,48,'surface',10);s.rect(196,184,166,40,'raised',8)
s.text(110,210,'Old Testament',16,'medium','muted',anchor='middle');s.text(279,210,'New Testament',16,'medium',anchor='middle')
s.label(24,262,'CHOOSE A BOOK')
names=['Matt','Mark','Luke','John','Acts','Rom','1 Cor','2 Cor','Gal','Eph','Phil','Col','1 Thess','2 Thess','1 Tim','2 Tim','Titus','Phlm','Heb','Jas','1 Pet','2 Pet','1 John','2 John','3 John','Jude','Rev']
for i,n in enumerate(names):
    x=24+(i%5)*70;y=280+(i//5)*52
    s.rect(x,y,62,44,'accent' if n=='John' else 'raised',8)
    s.text(x+31,y+28,n,12,'medium','inverse' if n=='John' else 'ink',anchor='middle')
s.label(24,634,'RECENT PASSAGES')
s.chip(24,652,102,'John 3:16',True);s.chip(134,652,96,'Psalm 23');s.chip(238,652,128,'Romans 8')
s.button(24,752,342,'Continue with John',True,'arrow');s.rect(145,826,100,4,'quiet',2);screens.append(s)

s=Screen('03-chapter-and-verse');s.header('Go to a passage',True,False);s.icon_button(334,42,'close')
s.rect(24,108,342,48,'raised',10,'border');s.icon(38,120,'search');s.text(74,138,'John 3:16',16)
s.chip(24,172,96,'Book: John');s.icon(130,178,'chevron','quiet');s.chip(164,172,88,'Chapter 3',True);s.chip(264,172,102,'Verse 16',True)
s.label(24,238,'CHAPTER')
for i in range(21):
    x=24+(i%7)*49;y=252+(i//7)*48
    s.rect(x,y,44,44,'accent' if i==2 else 'raised',8);s.text(x+22,y+28,str(i+1),16,'medium','inverse' if i==2 else 'ink',anchor='middle')
s.label(24,424,'VERSE · OPTIONAL')
for i in range(36):
    x=24+(i%7)*49;y=438+(i//7)*48
    s.rect(x,y,44,44,'accent' if i==15 else 'raised',8);s.text(x+22,y+28,str(i+1),16,'medium','inverse' if i==15 else 'ink',anchor='middle')
s.button(24,752,342,'Open John 3:16',True,'arrow');s.rect(145,826,100,4,'quiet',2);screens.append(s)

s=Screen('04-word-study-sheet');s.header();s.label(24,125,'JOHN 3:16')
s.text(24,165,'“For God so loved\nthe world…”',28,'serif',line=36)
s.rect(0,30,390,814,'#000000',opacity=.50)
s.rect(0,240,390,604,'surface',24);s.rect(171,252,48,4,'quiet',2)
s.label(24,286,'WORD STUDY · “LOVED”');s.icon_button(334,264,'close')
s.text(24,332,'ἀγαπάω',28,'greek');s.text(24,362,'agapaō · to love',16,'sans','muted');s.chip(24,382,66,'G25',True);s.chip(98,382,104,'Greek verb')
s.line(24,438,342);s.label(24,468,'FORM IN JOHN 3:16');s.text(24,496,'ἠγάπησεν · ēgapēsen',20,'greek')
s.text(24,526,'Aorist · active · indicative\n3rd person singular',16,'sans','muted',line=24)
s.line(24,574,342);s.label(24,604,'STRONG’S CONCORDANCE')
s.text(24,636,'To love; to regard with affection.',16,'sans',width=342,line=24)
s.text(24,668,'Illustrative entry · G25',12,'sans','quiet')
s.text(24,710,'Explore occurrences',16,'medium','accent');s.icon_button(322,684,'arrow','accent')
s.button(24,764,164,'Highlight');s.button(200,764,166,'Add note',True);s.rect(145,830,100,4,'quiet',2);screens.append(s)

s=Screen('05-note-and-highlight');s.header('Your note',True,False);s.label(24,124,'ATTACHED TO JOHN 3:16')
s.rect(24,144,342,144,'surface',12,'border');s.text(44,178,'“For God so loved the world,\nthat he gave his only begotten\nSon…”',20,'serif',line=30)
s.label(24,330,'HIGHLIGHT')
for i,c in enumerate(['#DCBC83','#B2C5AB','#9EBAC8','#C6A5A0']):
    s.rect(24+i*60,350,48,48,c,10)
    if i==0:s.icon(36,362,'check','inverse')
s.rect(24,426,342,250,'raised',12,'border');s.text(44,462,'Love takes the initiative',16,'medium')
s.text(44,502,'The movement begins with God’s love and becomes visible in the giving of his Son. Compare with 1 John 4:9.',16,'sans',width=302,line=26)
s.text(44,650,'Private on this device',12,'sans','muted')
s.text(24,710,'Follows the passage across translations.',12,'sans','muted')
s.button(24,752,104,'Cancel');s.button(140,752,226,'Save note',True);s.rect(145,826,100,4,'quiet',2);screens.append(s)

s=Screen('06-local-search');s.header('Ask your library',True,False);s.nav('Search')
s.rect(24,112,342,76,'raised',12,'border');s.icon(38,128,'search');s.text(74,140,'How does John describe\nGod’s love?',16,'sans',line=24)
s.chip(24,204,112,'On this device',True);s.chip(144,204,116,'KJV + resources')
s.rect(24,264,342,186,'surface',12,'border');s.icon(44,284,'spark','accent',20);s.label(76,301,'AI EXPLANATION')
s.text(44,338,'John describes God’s love through the giving of his Son. John 3:16 connects that gift with everlasting life; 1 John 4:9 echoes the same thought.',16,'sans',width=302,line=24)
s.label(24,484,'SCRIPTURE · KING JAMES VERSION');s.text(24,518,'John 3:16',16,'medium','accent');s.icon_button(322,490,'arrow','accent')
s.text(24,554,'For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.',20,'serif',width=342,line=28)
s.line(24,708,342);s.text(24,736,'1 John 4:9',16,'medium','accent');s.icon_button(322,710,'arrow','accent');screens.append(s)

s=Screen('07-genealogies');s.header('Genealogies',True,False);s.nav('Study');s.label(24,124,'ABRAHAM’S FAMILY');s.text(24,164,'Through generations',28,'serif')
s.chip(24,188,150,'4 generations');s.icon_button(322,184,'settings')
def person(x,y,w,name,ref,active=False):
    s.rect(x,y,w,64,'tint' if active else 'surface',10,'accent' if active else 'border');s.text(x+w/2,y+27,name,16,'medium','accent' if active else 'ink',anchor='middle');s.text(x+w/2,y+49,ref,12,'sans','muted',anchor='middle')
def conn(path):s.parts.append(f'<path d="{path}" fill="none" stroke="{COL["quiet"]}"/>')
person(107,252,176,'Abraham','Genesis 21:3');conn('M195 316v20');person(107,336,176,'Isaac','Genesis 25:26');conn('M195 400v20');person(107,420,176,'Jacob','Genesis 29:35');conn('M195 484v16M101 520v-20h188v20');person(24,520,154,'Judah','Genesis 29:35',True);person(212,520,154,'Joseph','Genesis 30:24')
s.rect(24,612,342,124,'surface',12,'border');s.label(44,640,'JACOB → JUDAH · PARENT & CHILD');s.text(44,670,'“…she called his name Judah…”',16,'sans',width=302);s.text(44,708,'Read Genesis 29:35',16,'medium','accent');s.icon(322,688,'arrow','accent');screens.append(s)

s=Screen('08-reading-preferences');s.header('Reading preferences',True,False);s.label(24,124,'MAKE ROOM FOR READING');s.text(24,166,'Your reading rhythm',28,'serif')
s.label(24,214,'APPEARANCE');s.button(24,234,164,'Study desk',True);s.button(200,234,166,'Paper')
s.label(24,322,'SCRIPTURE TYPEFACE');s.rect(24,340,342,48,'raised',10,'border');s.text(44,370,'Lora',16,'medium');s.icon(322,352,'down')
s.label(24,424,'TEXT SIZE');s.icon_button(24,440,'minus','muted','raised');s.text(195,470,'20 px',20,'medium',anchor='middle');s.icon_button(322,440,'plus','muted','raised')
s.line(24,508,342);s.text(24,546,'Verse numbers',16,'medium');s.rect(314,524,52,32,'accent',16);s.rect(338,528,24,24,'inverse',12)
s.rect(24,578,342,124,'surface',12,'border');s.label(44,608,'READING PREVIEW');s.text(44,644,'For God so loved the world…',20,'serif',width=302,line=28)
s.button(24,752,342,'Back to reading',True);s.rect(145,826,100,4,'quiet',2);screens.append(s)

saved=[]
for screen in screens:saved.append(screen.save())
# Board keeps every screen as a named vector group, with editable SVG text.
board=['<svg xmlns="http://www.w3.org/2000/svg" width="1768" height="1984" viewBox="0 0 1768 1984">','<rect width="1768" height="1984" fill="#101411"/>','<text x="56" y="60" font-family="IBM Plex Sans" font-size="28" fill="#ECEDE5">fontes / mobile study desk</text>','<text x="56" y="92" font-family="IBM Plex Sans" font-size="16" fill="#ABB3AA">390 × 844 · Lora + IBM Plex Sans · Phosphor Regular</text>']
for i,(screen,svg) in enumerate(zip(screens,saved)):
    x=56+(i%4)*422;y=156+(i//4)*904
    inner=re.sub(r'^<svg[^>]*>|</svg>$','',svg)
    board.append(f'<text x="{x}" y="{y-16}" font-family="IBM Plex Sans" font-size="16" fill="#ABB3AA">{screen.name.replace("-"," ")}</text><g id="{screen.name}" transform="translate({x} {y})">{inner}</g>')
board.append('</svg>');(ROOT/'mobile-mockups.svg').write_text(''.join(board))
(ROOT/'layout-audit.json').write_text(json.dumps({'screenCount':len(screens),'screenSize':[390,844],'textBoundsChecked':True,'fonts':['Lora','IBM Plex Sans'],'icons':'Phosphor Regular','screens':[dict(name=s.name,textNodes=len(s.audit)) for s in screens]},indent=2))
print(json.dumps({'screens':len(screens),'output':str(ROOT),'textBounds':'pass'}))
