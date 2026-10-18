"""Build original flat SVG book covers from the reviewed Covenant Forms briefs.
The four original style studies are retained. Every other composition is specified
explicitly below; no raster images, external SVG resources, randomness or fonts.
"""
from pathlib import Path
from html import escape
import json
import hashlib
ROOT=Path(__file__).resolve().parent
C='#F6EAD4'; I='#253D43'; R='#AD543C'; G='#D9AC58'; S='#8F9C83'; B='#6A939A'
def path(d,c=G,stroke=None,w=3):
 return f'<path d="{d}" fill="{c}"'+(f' stroke="{stroke}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"' if stroke else '')+'/>'
def circle(x,y,r,c=G):return f'<circle cx="{x}" cy="{y}" r="{r}" fill="{c}"/>'
def rect(x,y,w,h,c=G,rx=0):return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{c}"/>'
def line(d,c=C,w=4):return path(d,'none',c,w)
def star(x,y,r=9,c=G):return path(f'M{x} {y-r}l{r*.3} {r*.7} {r*.7} {r*.3}-{r*.7} {r*.3}-{r*.3} {r*.7}-{r*.3}-{r*.7}-{r*.7}-{r*.3} {r*.7}-{r*.3}Z',c)
def group(body,x=0,y=0,s=1,rotate=0):return f'<g transform="translate({x} {y}) rotate({rotate}) scale({s})">{body}</g>'
# Motifs are drawn around local origin. Scale and position are part of each scene.
def motif(k):
 if k=='tree':return path('M-9 90H9V-70H-9Z',R)+path('M0-120C-78-125-100-65-61-37-115 4-63 58-9 30V-50H9V30C63 58 115 4 61-37 100-65 78-125 0-120Z',S)+circle(-43,-34,7,G)+circle(45,-15,7,G)
 if k=='branch':return line('M0 95V-90',S,9)+''.join(path(f'M0 {y}C-70 {y-55}-75 {y-80}-15 {y-65}L0 {y}Z',S)+path(f'M0 {y+20}C70 {y-35}75 {y-60}15 {y-45}Z',S) for y in [-30,35])
 if k=='shoot':return rect(-7,0,14,70,S)+path('M0 15C-80 5-68-65 0-20 68-65 80 5 0 15Z',S)
 if k=='stump':return path('M-65 60H65L45-35H-45Z',R)+path('M-45-35Q0-60 45-35Q0-10-45-35Z',G)+line('M-26-34Q0-46 26-34',R,3)
 if k=='mountain':return path('M-110 85 0-95 110 85Z',S)+path('M-34-38 0-95 35-38 12-44 0-28-13-44Z',C)
 if k=='tent':return path('M-100 75 0-85 100 75Z',R)+path('M0-42 0 75H52Z',C)+line('M-111 75H111',G,3)
 if k=='scroll':return path('M-65-95H76V70H-65Z',C)+path('M-65-95C-105-95-105-50-65-50H-42V-75H-65Z',G)+path('M76 70C110 70 110 105 76 105H-59V83H76Z',G)+''.join(rect(-35,y,75,4,B) for y in [-42,-15,12,39])
 if k=='book':return path('M0-65C-38-90-70-90-100-80V70C-60 60-30 65 0 90 30 65 60 60 100 70V-80C70-90 38-90 0-65Z',C)+line('M0-65V90',R,4)+''.join(line(f'M-80 {y}Q-40 {y-5}-17 {y+15}M17 {y+15}Q40 {y-5}80 {y}',B,3) for y in [-35,-5,25])
 if k=='lamp':return path('M-75 15H50C38 70-50 80-75 15Z',G)+path('M40 15C120-20 120 65 48 48L48 34C96 44 94 0 48 30Z',G)+path('M-20 8C-60-32 4-48-15-92 60-24 10-14-20 8Z',R)
 if k=='flame':return path('M0 100C-110 44-85-30-20-95-35-24 34-30 22-125 140-2 80 75 0 100Z',G)+path('M0 82C-50 42-5 13 0-23 50 34 45 56 0 82Z',R)
 if k=='cloud':return path('M-100 22C-130-26-85-68-42-47-25-105 55-92 68-45 125-70 150 10 100 30Z',C)
 if k=='crown':return path('M-85-50-48-15 0-70 48-15 85-50 65 65H-65Z',G)+rect(-67,76,134,15,G)+circle(0,-84,8,G)
 if k=='split-crown':return path('M-95-60-58-20-15-70-27 75H-78Z',G)+path('M17-70 58-20 95-60 78 75H27Z',G)
 if k=='temple':return path('M-105 100V-65H105V100H40V-20H-40V100Z',R)+rect(-120,-90,240,20,G)+rect(-110,100,220,18,G)+rect(-93,-45,20,145,C)+rect(73,-45,20,145,C)+rect(-100,-55,34,12,G)+rect(66,-55,34,12,G)
 if k=='gate':return path('M-110 100V-70H-65V-95H65V-70H110V100H40V-25A40 40 0 0 0-40-25V100Z',R)+rect(-120,100,240,16,G)
 if k=='door':return path('M-85 110V-50a85 85 0 0 1 170 0V110H45V-48a45 45 0 0 0-90 0V110Z',R)+path('M-30-45 30-76V107L-30 83Z',G)
 if k=='wall':return ''.join(rect(x+(row%2)*-25,y,75,35,G) for row,y in enumerate([-45,-5,35,75]) for x in [-110,-30,50])+path('M-112-60V-100H-80V-60M-16-60V-100H16V-60M80-60V-100H112V-60',R)
 if k=='ruin':return path('M-115 100V-15H-80V-85H-35V-10L-5-45 26 15 65-15 90 22 115 10V100Z',R)+rect(-80,2,24,30,I)+rect(43,44,25,30,I)
 if k=='foundation':return ''.join(rect(x,y,75,36,G) for x,y in [(-120,65),(-37,65),(46,65),(-79,21),(4,21)])
 if k=='altar':return rect(-80,-35,160,110,R)+path('M-80-35V-65H-60V-35M60-35V-65H80V-35',G)+rect(-95,75,190,17,G)
 if k=='basin':return path('M-100-20H100Q85 65 0 65T-100-20Z',B)+path('M-15 65H15V100H55V113H-55V100H-15Z',G)+path('M-75-20Q0 5 75-20',C)
 if k=='veil':return path('M-120-120H-12L-20 20-95 105H-120ZM120-120H12L20 20 95 105H120Z',R)+line('M-100-100-90 64M-55-100-48 5M100-100 90 64M55-100 48 5',G,3)
 if k=='jar':return path('M-40-110H40V-65C130-30 120 95 55 110H-55C-120 95-130-30-40-65Z',R)+path('M-18-56 15-20-12 15 17 57-1 110',G)+rect(-45,-120,90,15,G)
 if k=='vessel':return path('M-90-70H90L60 80Q0 120-60 80Z',R)+path('M-90-70Q0-110 90-70Q0-35-90-70Z',G)+path('M-67-70Q0-94 67-70Q0-52-67-70Z',I)
 if k=='harp':return path('M-70-100H70V75Q60 120 0 120T-70 75Z',G)+path('M-45-75H45V65Q30 90 0 90T-45 65Z',I)+''.join(rect(x,-75,3,150,C) for x in [-30,-15,0,15,30])
 if k=='staff':return line('M-10 115V-65C-10-135 80-135 80-65',G,13)
 if k=='scales':return rect(-6,-85,12,170,G)+rect(-65,85,130,13,G)+line('M-90-60H90M-70-60-100 5M-70-60-40 5M70-60 40 5M70-60 100 5',G,4)+path('M-110 5H-30Q-70 65-110 5ZM30 5H110Q70 65 30 5Z',B)
 if k=='seed':return path('M0 85C-100 20-85-65 0-95 85-65 100 20 0 85Z',G)+line('M0 58V-70',R,3)
 if k=='heart':return path('M0 100C-140 5-95-110-22-65L0-42 22-65C95-110 140 5 0 100Z',R)+path('M0 65V-15',G)
 if k=='wheat':return line('M0 120V-95',G,5)+''.join(path(f'M0 {y}C-60 {y-30}-55 {y-60}0 {y-23} 55 {y-60}60 {y-30}0 {y}Z',G) for y in [-50,0,50])
 if k=='table':return path('M-120-15 0-60 120-15 0 25Z',G)+path('M-95 5V100H-75V15M75 15V100H95V5',R)+path('M-7 25H7V100H-7Z',R)
 if k=='cup':return path('M-70-95H70L50-15Q0 45-50-15Z',G)+rect(-7,15,14,75,G)+rect(-55,90,110,12,G)+path('M-55-82H55L43-45H-43Z',R)
 if k=='chain':return line('M-100-55-45 0M45 0 100 55',G,16)+path('M-96-82Q-145-45-100-7L-57 35Q-12 65 20 15L3 0Q-20 34-45 11L-85-25Q-110-48-80-64Z',G)+path('M96 82Q145 45 100 7L57-35Q12-65-20-15L-3 0Q20-34 45-11L85 25Q110 48 80 64Z',G)
 if k=='yoke':return path('M-125-45H-15V-15H-125ZM15-45H125V-15H15Z',R)+line('M-90-15V50Q-60 100-30 50V-15M30-15V50Q60 100 90 50V-15',G,13)
 if k=='anchor':return line('M0-80V90M-40-20H40M-85 30Q-60 100 0 100T85 30M-85 30-85 65M85 30 85 65',G,10)+circle(0,-105,23,G)+circle(0,-105,11,I)
 if k=='tower':return path('M-60 110-42-100H42L60 110Z',R)+path('M-60-100V-130H-30V-100H-15V-130H15V-100H30V-130H60V-100Z',G)+rect(-17,-45,34,45,I)+rect(-82,110,164,15,G)
 if k=='plumb':return line('M0-130V60',C,4)+path('M0 105-28 55 0 15 28 55Z',G)+rect(-90,-130,180,13,R)
 if k=='fish':return path('M-125 0Q-30-130 85-20L125-65V65L85 20Q-30 130-125 0Z',B)+circle(-70,-8,5,I)+path('M-35 18 3-13 6 34Z',C)
 if k=='nest':return path('M-100-10Q0 100 100-10L75 65Q0 120-75 65Z',G)+line('M-90 0 70 70M-45-5 95 35M-95 35 45-5M-70 70 90 0',R,4)+circle(-18,0,21,C)+circle(25,-3,20,C)
 if k=='lion':return path('M-110 70V-10C-110-80-35-100-5-50L35-45 60-12 25 5 10 62-20 82H-90Z',G)+path('M-60-68Q-140-22-80 63L-35 40V-45Z',R)+circle(3,-30,4,I)
 if k=='hands':return path('M-130 95-75 15-15 48 0 75-60 65-100 135ZM130 95 75 15 15 48 0 75 60 65 100 135Z',S)+path('M-75 15-55-30-37-12-45 32M75 15 55-30 37-12 45 32',S)
 if k=='bone':return line('M-65 70 65-70',C,18)+''.join(circle(x,y,19,C) for x,y in [(-80,60),(-55,85),(55,-85),(80,-60)])
 if k=='cross':return rect(-12,-120,24,230,G)+rect(-80,-70,160,24,G)
 if k=='lamb':return path('M-85 20C-115-45-40-75 15-45L70-60 100-28 75-5 55 5 45 75H25L15 20H-40L-45 75H-65L-70 20Z',C)+circle(72,-35,4,I)
 if k=='stone':return path('M-100 70-70-50 22-90 100-20 78 95Z',S)+path('M-70-50 22-90 13 20Z',G)
 if k=='scepter':return rect(-7,-75,14,200,G)+circle(0,-100,27,G)+path('M-27-100 0-137 27-100Z',G)
 if k=='sun':return circle(0,0,60,G)
 if k=='whirlwind':return ''.join(path(f'M{-115+n*15} {-100+n*30}Q0 {-145+n*30} {115-n*15} {-100+n*30}L{100-n*15} {-86+n*30}Q0 {-127+n*30} {-100+n*15} {-86+n*30}Z',B) for n in range(6))
 if k=='wing':return path('M0 65C-10-75-160-105-160-105-147-10-90 75 0 65 90 75 147-10 160-105 160-105 10-75 0 65Z',C)
 if k=='weave':return line('M-90-100 55 100M-55-100 90 100',G,22)+line('M90-100-55 100M55-100-90 100',B,22)+path('M-25-6 0-40 25-6 0 29Z',G)
 raise ValueError(k)
def m(k,x=300,y=350,s=1,rotate=0):return group(motif(k),x,y,s,rotate)
def sky_stars():return ''.join(star(x,y,r) for x,y,r in [(145,170,6),(430,175,8),(210,100,5),(475,270,5)])
def river(d='M290 380C140 410 420 453 260 516H350C500 450 205 425 310 380Z'):return path(d,C)
def terrain(c=S,d='M64 445Q180 350 300 418T536 420V516H64Z'):return path(d,c)
def road():return path('M282 305H318L415 516H185Z',C)
def stars():return sky_stars()
def rains():return ''.join(line(f'M{x} {y}l-12 27',B,4) for x,y in [(140,210),(220,175),(390,180),(470,220),(190,260),(430,270)])
def foundation_strip():return rect(0,516,600,124,R)
def footer_detail(osis):
 if osis in ['Num','Ps','Isa','Ezek','Amos','Titus','Jas','Rev']:
  return path('M260 516H350C230 560 400 588 300 640H210C370 585 160 555 260 516Z',C)
 if osis in ['Deut','Josh','2Kgs','2Chr','Nah','Matt','Mark','Acts','Rom','1Pet','1John','3John','2Tim']:
  return path('M250 516H350L430 640H170Z',C)
 if osis in ['Lev','1Kgs','Ezra','Neh','Hag','Zech','Eph','Heb','1Tim']:
  return ''.join(rect(x,551,72,13,G) for x in [100,182,264,346,428])+''.join(rect(x,577,72,13,G) for x in [141,223,305,387])
 if osis in ['1Chr','Prov','Col','Gal','Joel','2Pet','Hos','Song']:
  return line('M300 516V596M300 550 240 580M300 550 360 580M300 575 265 607M300 575 335 607',G,5)
 if osis in ['Job','Judg','Eccl','Hab','Lam','Zeph','Obad']:
  return line('M64 576Q180 550 300 576T536 576M64 594Q180 568 300 594T536 594',G,4)
 return ''.join(star(x,575,r,G) for x,r in [(245,5),(300,10),(355,5)])
def scene(osis):
 # Each tuple is (sky layer, land layer, narrative foreground, footer).
 if osis=='Lev':return '',terrain(B),m('veil',300,275,1.55)+m('altar',300,401,.72)+m('basin',446,458,.35),''
 if osis=='Num':return m('cloud',300,170,.68),terrain(B)+path('M64 468Q290 380 536 470V516H64Z',S),river('M290 280C200 340 400 370 280 420S200 480 250 516H320C260 480 390 450 340 405S260 335 310 280Z')+m('tent',170,430,.45)+m('tent',410,455,.38)+m('tent',300,355,.23),''
 if osis=='Deut':return m('sun',410,170,.65),terrain(S)+m('mountain',300,315,1.4),path('M290 420 135 516H200L300 450 440 516H495L310 420Z',C)+m('scroll',300,348,.60),''
 if osis=='Josh':return stars(),terrain(S)+path('M64 420H536V470H64Z',B),road()+''.join(rect(x,y,19,26,G,3) for x,y in [(172,433),(198,433),(224,433),(250,433),(276,433),(302,433),(328,433),(354,433),(380,433),(406,433),(432,433),(458,433)]),''
 if osis=='Judg':return m('sun',440,180,.4),terrain(B)+path('M64 350 290 465 536 355V516H64Z',S),path('M260 225A115 115 0 1 0 399 360L375 344A88 88 0 1 1 266 253Z',G)+path('M368 315 413 360 352 370Z',G),''
 if osis=='1Sam':return stars()+m('crown',300,220,.65),terrain(B),m('staff',390,390,.74)+m('lamp',230,420,.64),''
 if osis=='2Sam':return m('crown',300,215,.95),terrain(B),m('foundation',300,414,1.1)+path('M292 395 270 442 306 458 282 516H310L332 449 299 435 318 395Z',I)+m('branch',360,376,.64),''
 if osis=='1Kgs':return '',terrain(S),m('temple',190,330,.72)+m('split-crown',410,335,.65)+m('flame',300,447,.34),''
 if osis=='2Kgs':return stars(),terrain(B),m('ruin',300,337,1.15)+path('M285 370 310 370 435 516H240Z',C)+m('shoot',170,446,.48),''
 if osis=='1Chr':return m('sun',300,175,.6),terrain(B),m('harp',300,324,.83)+''.join(line(f'M300 410Q{x} 430 {x} 495',G,5) for x in [155,210,270,330,390,445])+m('foundation',300,464,.42),''
 if osis=='2Chr':return m('sun',300,205,.9),terrain(B),m('temple',300,340,1.1)+path('M285 362H315L400 516H200Z',C),''
 if osis=='Ezra':return stars(),terrain(S),m('scroll',300,225,.61)+m('foundation',300,441,1.25)+m('altar',300,389,.48),''
 if osis=='Neh':return m('sun',425,205,.55),terrain(B),m('wall',160,392,.68)+m('wall',442,392,.68)+m('gate',300,380,.74)+path('M275 425H325L380 516H220Z',C),''
 if osis=='Esth':return stars(),terrain(B),m('door',300,300,1.15)+m('scroll',422,435,.43)+m('scepter',215,335,.76,32),''
 if osis=='Job':return stars(),terrain(B)+path('M64 490Q160 400 280 480T536 480V516H64Z',R),m('whirlwind',300,275,1.2)+path('M185 516 230 463 275 516Z',G),''
 if osis=='Ps':return m('sun',410,165,.62)+star(165,180,12,C),terrain(S)+path('M64 420H536V516H64Z',B),m('harp',300,320,1.1)+river(),''
 if osis=='Prov':return stars(),terrain(B),path('M285 392 130 516H200L300 427 410 516H480L315 392Z',C)+m('tree',300,350,.87)+m('scales',300,345,.45),''
 if osis=='Eccl':return m('sun',300,174,.8),terrain(B),path('M190 305A110 110 0 0 1 410 305H390A90 90 0 0 0 210 305ZM410 320A110 110 0 0 1 190 320H210A90 90 0 0 0 390 320Z',S)+m('vessel',300,428,.54),''
 if osis=='Song':return stars(),terrain(S),m('gate',300,350,.92)+m('branch',165,347,.76,-18)+m('branch',435,347,.76,18)+''.join(circle(x,y,8,G) for x,y in [(170,290),(435,320),(400,260),(205,370)]),''
 if osis=='Isa':return stars(),terrain(B),road()+m('stump',300,419,.8)+m('shoot',300,355,.7)+circle(170,260,23,R)+circle(170,260,11,G),''
 if osis=='Jer':return m('sun',410,180,.45),terrain(B),m('jar',250,373,.92)+m('branch',395,310,.85,-20)+group(motif('heart').replace(R,C),250,366,.32),''
 if osis=='Lam':return m('sun',430,284,.6),terrain(B),m('ruin',235,365,1.1)+path('M430 375C340 442 375 470 420 470 480 470 485 422 430 375Z',C),''
 if osis=='Ezek':return stars(),terrain(R),river('M310 235C190 320 430 357 270 516H370C505 350 290 310 330 235Z')+m('bone',165,425,.52,-15)+m('bone',238,340,.32,20)+m('branch',435,390,.65),''
 if osis=='Dan':return stars(),terrain(B),m('flame',300,240,.6)+m('lion',175,390,.68)+'<g transform="translate(425 390) scale(-.68 .68)">'+motif('lion')+'</g>'+m('stone',300,448,.5),''
 if osis=='Hos':return stars(),terrain(B),m('door',330,325,1.1)+line('M110 480C60 285 290 290 170 430S465 520 400 270',S,16)+m('shoot',390,260,.5)+circle(392,235,14,G),''
 if osis=='Joel':return m('cloud',300,140,.62)+rains(),terrain(R)+path('M300 420Q420 355 536 425V516H300Z',S),''.join(line(f'M{x} 500V{y}',G,5) for x,y in [(110,440),(170,425),(230,445)])+group(motif('shoot').replace(S,G),355,442,.5)+group(motif('shoot').replace(S,G),440,416,.65),''
 if osis=='Amos':return stars(),terrain(S),path('M170 300C80 400 350 410 230 516H380C520 405 240 350 210 300Z',B)+m('plumb',320,275,.88)+m('tent',445,422,.44),''
 if osis=='Obad':return m('nest',270,205,.48),path('M64 516 175 270 250 250 320 380 390 338 536 516Z',S),m('crown',400,441,.45,20)+path('M270 240 308 292 295 300 255 247Z',G),''
 if osis=='Jonah':return m('gate',420,260,.42),terrain(S)+path('M64 360Q240 405 536 350V516H64Z',B),m('fish',250,415,1.05)+m('branch',145,276,.7),''
 if osis=='Mic':return stars(),terrain(B),m('scales',240,320,.83)+m('staff',390,336,.78)+m('branch',300,445,.42,-50),''
 if osis=='Nah':return stars(),terrain(B),m('wall',300,265,.86)+road()+m('yoke',300,405,.92),''
 if osis=='Hab':return m('sun',435,285,.48),terrain(R),m('tower',220,340,1.05)+''.join(line(f'M{x} 516V{y}',G,4) for x,y in [(360,440),(400,454),(440,431),(480,455)]),''
 if osis=='Zeph':return stars(),terrain(B),m('ruin',330,320,1.2)+m('tent',220,425,.62)+m('seed',220,439,.24)+line('M120 235Q210 165 300 205M140 260Q220 205 290 230',G,5),''
 if osis=='Hag':return m('sun',420,200,.5),terrain(B),m('foundation',300,438,1.2)+rect(205,238,25,180,C)+rect(375,238,25,180,C)+path('M185 230 300 155 415 230 415 247 300 180 185 247Z',G)+path('M280 438H320V205H280Z',G),''
 if osis=='Zech':return stars(),terrain(B),m('gate',300,419,.55)+m('branch',160,305,.85,-18)+m('branch',440,305,.85,18)+rect(290,255,20,180,G)+''.join(m('lamp',x,y,.25) for x,y in [(205,240),(300,205),(395,240)])+line('M205 255V290H395V255',G,7),''
 if osis=='Mal':return m('sun',300,195,.78)+''.join(line(f'M300 195 {x} 350',G,4) for x in [125,190,245,355,410,475]),terrain(B),m('vessel',300,400,.9)+m('flame',300,325,.45),''
 if osis=='Matt':return m('crown',300,190,.92),terrain(B)+m('mountain',300,362,1.3),road()+m('gate',300,437,.62),''
 if osis=='Mark':return stars(),terrain(S),path('M125 516 300 310 355 260 385 280 340 335 240 516Z',C)+m('basin',200,415,.48)+m('cross',377,273,.62),''
 if osis=='Luke':return stars(),terrain(B),m('door',300,285,1.5)+m('table',300,411,.95)+m('lamp',300,351,.38),''
 if osis=='Acts':return m('flame',300,195,.8),terrain(B),path('M285 310H315V360L475 516H410L300 400 190 516H125L285 360Z',C)+path('M290 390H310V516H290Z',G),''
 if osis=='Rom':return stars(),terrain(B),road()+m('chain',300,421,.82)+m('branch',300,278,.84)+line('M287 318 316 342',C,5),''
 if osis=='1Cor':return stars(),terrain(B),m('foundation',300,407,1.25)+m('cup',300,300,.84)+m('seed',300,480,.28),''
 if osis=='2Cor':return stars(),terrain(B),m('hands',300,390,1.25)+m('jar',300,317,1.05)+circle(300,306,38,G)+path('M297 270H306V413H297Z',C),''
 if osis=='Gal':return stars(),terrain(B),m('yoke',300,416,1.23)+m('branch',300,290,1.05)+circle(254,250,10,G)+circle(350,300,10,G),''
 if osis=='Eph':return m('sun',300,185,.8),terrain(B),m('foundation',300,421,1.35)+m('door',300,310,1.07)+rect(265,368,70,58,G),''
 if osis=='Phil':return m('sun',420,178,.73),terrain(B),path('M65 400H170V440H245V470H300V425H355V375H410V310H465V255H536V280H490V335H435V400H380V450H325V495H220V465H145V425H65Z',G),''
 if osis=='Col':return stars(),terrain(B),m('branch',300,331,1.1)+m('crown',300,224,.88)+''.join(line(f'M300 425Q{x} 450 {x} 510',G,7) for x in [170,235,300,365,430]),''
 if osis=='1Thess':return m('sun',430,285,.43),terrain(B),m('tent',300,384,1.3)+m('lamp',300,408,.65),''
 if osis=='2Thess':return m('sun',430,285,.43),terrain(S)+''.join(line(f'M{x} 516 300 370',R,8) for x in [70,150,230,370,450,530]),m('anchor',275,390,.82)+m('lamp',275,280,.64),''
 if osis=='1Tim':return stars(),terrain(B),m('door',300,285,1.5)+m('book',300,410,.95)+m('lamp',300,308,.5),''
 if osis=='2Tim':return stars(),terrain(B),m('scroll',240,371,.95)+m('flame',245,220,.68)+path('M310 390 370 516H435L330 365Z',C)+rect(385,285,8,120,G)+path('M393 285H465L440 312 465 335H393Z',G),''
 if osis=='Titus':return stars(),terrain(B),m('door',393,340,.8)+river('M165 400C60 435 325 460 185 516H295C400 448 130 425 185 400Z')+m('branch',185,318,.9),''
 if osis=='Phlm':return stars(),terrain(B),m('door',160,290,.75)+m('door',440,290,.75)+m('table',300,365,.95)+m('chain',300,457,.57),''
 if osis=='Heb':return stars(),terrain(B),m('veil',300,255,1.6)+m('door',300,285,.75)+m('anchor',300,426,.72),''
 if osis=='Jas':return stars(),terrain(B),path('M282 390 120 516H195L300 430 405 516H480L318 390Z',C)+m('tree',300,330,1.04)+river('M160 380C50 420 320 460 185 516H225C390 450 100 420 180 380Z'),''
 if osis=='1Pet':return m('sun',430,235,.53),terrain(B),m('foundation',300,417,1.27)+m('tent',300,328,1.2)+path('M290 516H310V440H290Z',C),''
 if osis=='2Pet':return star(300,155,32,G),terrain(S)+path('M64 465Q270 405 536 460V516H64Z',B),m('branch',300,350,1.2),''
 if osis=='1John':return m('sun',300,180,.75),terrain(B),road()+m('hands',300,355,1.35)+m('heart',300,290,.75),''
 if osis=='2John':return stars(),terrain(B),m('door',300,300,1.15)+m('weave',300,250,.6)+m('lamp',300,437,.52),''
 if osis=='3John':return stars(),terrain(B),m('door',300,280,1.45)+path('M282 370H318L400 516H200Z',C)+m('table',300,370,.75),''
 if osis=='Jude':return stars(),terrain(B),path('M170 450V295a130 130 0 0 1 260 0V450H405V295a105 105 0 0 0-210 0V450Z',S)+m('flame',300,340,.84)+path('M290 145H310V215H290Z',C),''
 if osis=='Rev':return stars(),terrain(B),m('gate',300,290,1.17)+river('M290 350C150 405 460 445 250 516H350C540 448 250 400 310 350Z')+m('tree',435,413,.62)+m('lamb',190,437,.62),''
 raise ValueError(osis)
def build():
 data=json.loads((ROOT/'collection/books.json').read_text())
 for b in data['books']:
  if b['osis'] in ['Gen','Exod','Ruth','John']:
   b['art_status']='SVG cover · retained style study';continue
  sky,land,foreground,footer=scene(b['osis'])
  slug=b['osis'].lower()+'-covenant-forms.svg'
  desc=b['composition']+' Themes: '+', '.join(b['themes'])+'.'
  svg=f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 640" role="img" aria-labelledby="title desc"><title id="title">{escape(b['name'])} — {escape(b['title'])}</title><desc id="desc">{escape(desc)}</desc><defs><clipPath id="arch"><path d="M64 516V260a236 236 0 0 1 472 0v256Z"/></clipPath></defs><path fill="{C}" d="M0 0h600v640H0z"/><g clip-path="url(#arch)"><path fill="{I}" d="M64 24h472v492H64z"/>{sky}{land}{foreground}</g>{foundation_strip()}{footer_detail(b["osis"])}{footer}</svg>'''
  (ROOT/'art'/slug).write_text(svg+'\n')
  b['art']='../art/'+slug;b['art_status']='SVG cover · first edition'
 for b in data['books']:
  payload=(ROOT/'collection'/b['art']).read_bytes();b['art_sha256']=hashlib.sha256(payload).hexdigest();b['art_bytes']=len(payload)
 data['status']='Complete 66-book first-edition SVG art collection; introduction boundaries still under review'
 (ROOT/'collection/books.json').write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n')
if __name__=='__main__':build()
