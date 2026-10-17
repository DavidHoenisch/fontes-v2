"""Collect the current Fontes UI assets for offline native implementation."""
from pathlib import Path
import hashlib, json, re, shutil, subprocess, urllib.request

ROOT = Path(__file__).resolve().parent.parent
DEST = ROOT / 'assets/ui'
MOBILE = ROOT / 'design/mobile/assets'
for folder in ['icons/phosphor/regular', 'icons/png/24', 'icons/png/48', 'fonts', 'licenses', 'previews']:
    (DEST/folder).mkdir(parents=True, exist_ok=True)

roles = {'book':'book-open','search':'magnifying-glass','notes':'note-pencil','tree':'tree-structure','settings':'sliders-horizontal','chevron':'caret-right','down':'caret-down','close':'x','arrow':'arrow-right','back':'arrow-left','bookmark':'bookmark-simple','spark':'sparkle','check':'check','copy':'copy','plus':'plus','minus':'minus','wifi':'wifi-high','battery':'battery-full'}
sources = json.loads((MOBILE/'phosphor-regular.json').read_text())
icons = []
for role, name in roles.items():
    url = f'https://raw.githubusercontent.com/phosphor-icons/core/main/assets/regular/{name}.svg'
    source = sources.get(role) or urllib.request.urlopen(url).read().decode()
    # Native SVG renderers can tint a white icon; no CSS currentColor is needed.
    svg = source.replace('<svg ', '<svg width="24" height="24" ').replace('currentColor', '#FFFFFF')
    target = DEST/f'icons/phosphor/regular/{name}.svg'
    target.write_text(svg)
    icons.append({'role':role,'name':name,'svg':str(target.relative_to(DEST)), 'png24':f'icons/png/24/{name}.png','png48':f'icons/png/48/{name}.png','source':url,'license':'MIT'})

font_specs = [
    ('IBMPlexSans-Regular.ttf','IBM Plex Sans','Regular','ui_body','IBM-PLEX-OFL.txt'),
    ('IBMPlexSans-Medium.ttf','IBM Plex Sans','Medium','ui_label','IBM-PLEX-OFL.txt'),
    ('Lora-Regular.ttf','Lora','Regular','scripture','LORA-OFL.txt'),
    ('NotoSerif-Regular.ttf','Noto Serif','Regular','greek_fallback','NOTO-SERIF-OFL.txt'),
    ('NotoSerifHebrew-Regular.ttf','Noto Serif Hebrew','Regular','hebrew_fallback','NOTO-SERIF-HEBREW-OFL.txt'),
]
hebrew_url = 'https://fonts.gstatic.com/s/notoserifhebrew/v30/k3k0o9MMPvpLmixYH7euCwmkS9DohjX1-kRyiqyBqIxnoLbp93i9IKrXKF_qVAwSAG8.ttf'
hebrew_target=DEST/'fonts/NotoSerifHebrew-Regular.ttf'
if not hebrew_target.exists(): hebrew_target.write_bytes(urllib.request.urlopen(hebrew_url).read())
license_target=DEST/'licenses/NOTO-SERIF-HEBREW-OFL.txt'
if not license_target.exists(): license_target.write_bytes(urllib.request.urlopen('https://raw.githubusercontent.com/google/fonts/main/ofl/notoserifhebrew/OFL.txt').read())
font_sources = {
    'IBMPlexSans-Regular.ttf':'https://fonts.gstatic.com/s/ibmplexsans/v23/zYXGKVElMYYaJe8bpLHnCwDKr932-G7dytD-Dmu1swZSAXcomDVmadSD6llzAA.ttf',
    'IBMPlexSans-Medium.ttf':'https://fonts.gstatic.com/s/ibmplexsans/v23/zYXGKVElMYYaJe8bpLHnCwDKr932-G7dytD-Dmu1swZSAXcomDVmadSD2FlzAA.ttf',
    'Lora-Regular.ttf':'https://fonts.gstatic.com/s/lora/v37/0QI6MX1D_JOuGQbT0gvTJPa787weuyJG.ttf',
    'NotoSerif-Regular.ttf':'https://fonts.gstatic.com/s/notoserif/v33/ga6iaw1J5X9T9RW6j9bNVls-hfgvz8JcMofYTa32J4wsL2JAlAhZqFCjwA.ttf',
    'NotoSerifHebrew-Regular.ttf':hebrew_url,
}
fonts=[]
for filename,family,weight,role,license_name in font_specs:
    if (MOBILE/'fonts'/filename).exists(): shutil.copy2(MOBILE/'fonts'/filename, DEST/'fonts'/filename)
    if (MOBILE/'fonts'/license_name).exists(): shutil.copy2(MOBILE/'fonts'/license_name,DEST/'licenses'/license_name)
    fonts.append(dict(family=family,weight=weight,role=role,path='fonts/'+filename,source=font_sources[filename],license='OFL-1.1',licenseFile='licenses/'+license_name))
shutil.copy2(MOBILE/'PHOSPHOR-LICENSE.txt', DEST/'licenses/PHOSPHOR-LICENSE.txt')

tokens = {
    'name':'Fontes paper and sea glass','version':1,
    'colors':dict(desk='#FAF8F3',sidebar='#F1F3EF',surface='#FFFFFF',raised='#EFF2EE',border='#D8E0DB',ink='#202F30',muted='#526768',quiet='#627678',accent='#24676B',tint='#E0EEEA',inverse='#FFFFFF',green='#46715C',error='#A43D37',scrim='#203234'),
    'highlightColors':dict(amber='#F7E8BA',sage='#DFEBDD',blue='#DEEAF5',rose='#F6E1DE'),
    'spacing':[2,4,6,8,12,16,20,24,32,40,48,64],
    'radii':dict(control=12,card=16,dialog=16,mobileSheet=24),
    'desktop':dict(viewport=[1440,960],sidebarWidth=208,navigationRowHeight=36,sidebarItemGap=4,navigationIconGap=8,navigationPaddingVertical=8,savedPassagePaddingVertical=6,savedPassageContentGap=2,navigationSectionSpacer=16,smallSectionSpacer=4,toolbarHeight=72,windowBarHeight=64,readerWidth=800,studyPanelWidth=432,iconSize=20),
    'mobile':dict(viewport=[390,844],horizontalPadding=24,contentWidth=342,tapTargetMinimum=44,actionHeight=48,navigationRowHeight=64,iconSize=22,verseHorizontalPadding=18,verseContentWidth=354,lookupActionHeight=64,referenceCellSize=44,referenceCellGap=5),
    'typography':{
        'desktop':{'uiBody':dict(family='IBM Plex Sans',weight=400,size=15,lineHeight=22),'uiLabel':dict(family='IBM Plex Sans',weight=500,size=14,lineHeight=20),'caption':dict(family='IBM Plex Sans',weight=400,size=12,lineHeight=18),'title':dict(family='IBM Plex Sans',weight=500,size=24,lineHeight=32),'passage':dict(family='Lora',weight=400,size=24,lineHeight=39.12),'readingHeading':dict(family='Lora',weight=400,size=38,lineHeight=44.08),'originalWordHeading':dict(family='Noto Serif',fallback='Noto Serif Hebrew',weight=400,size=53,lineHeight=84)},
        'mobile':{'uiBody':dict(family='IBM Plex Sans',weight=400,size=16,lineHeight=24),'uiLabel':dict(family='IBM Plex Sans',weight=500,size=16,lineHeight=24),'caption':dict(family='IBM Plex Sans',weight=400,size=12,lineHeight=18),'title':dict(family='IBM Plex Sans',weight=500,size=20,lineHeight=28),'passage':dict(family='Lora',weight=400,size=24,lineHeight=39.12),'readingHeading':dict(family='Lora',weight=400,size=42,lineHeight=48.72),'originalWordHeading':dict(family='Noto Serif',fallback='Noto Serif Hebrew',weight=400,size=53,lineHeight=84)},
        'originalLanguageFallback':['Noto Serif','Noto Serif Hebrew'],
    },
    'status':{'palette':'Working light direction for owner review; supersedes dark study-desk default','sidebarSpacing':'Approved locally; Figma changes pending tool quota','mobileMockups':'Local SVG and PNG mockups','desktopMockups':'Figma only; desktop frame export blocked by tool quota'},
}
tokens['typography']['mobile']['uiBody'].update(size=15,lineHeight=22)
tokens['typography']['mobile']['uiLabel'].update(size=14,lineHeight=20)
for platform in ['desktop','mobile']:
    tokens['typography'][platform].update(editorialTitle=dict(family='Lora',weight=400,size=30,lineHeight=35.4),cardTitle=dict(family='Lora',weight=400,size=22,lineHeight=25.96),eyebrow=dict(family='IBM Plex Sans',weight=500,size=9,lineHeight=12.5,letterSpacing=1.3))
catalog=json.loads((DEST/'themes.json').read_text())
base=next(t for t in catalog['themes'] if t['id']==catalog['default'])
roles=dict(desk='page',sidebar='chrome',surface='surface',raised='raised',border='border',ink='ink',muted='muted',quiet='muted',accent='accent',tint='tint',inverse='on_accent',green='green',error='error',scrim='scrim')
tokens.update(name='Fontes selectable reading themes',themeFile='themes.json',defaultTheme=catalog['default'])
tokens['colors']={k:base['colors'][v] for k,v in roles.items()}
tokens['highlightColors']=base['highlights']
tokens['status']['palette']='Seven owner-selected local palettes; default sea glass. Native device verification remains separate.'
(DEST/'tokens.json').write_text(json.dumps(tokens,indent=2)+'\n')

rs=['//! Embedded Fontes UI assets and design constants. No network access required.','//! Include this file as a module; colors are sRGB RGBA bytes.','']
for name,h in tokens['colors'].items():
    rgb=[int(h[i:i+2],16) for i in (1,3,5)]+[255]
    rs.append(f'pub const COLOR_{name.upper()}: [u8; 4] = {rgb};')
for name,h in tokens['highlightColors'].items():
    rgb=[int(h[i:i+2],16) for i in (1,3,5)]+[255]
    rs.append(f'pub const HIGHLIGHT_{name.upper()}: [u8; 4] = {rgb};')
for name,n in tokens['desktop'].items():
    if isinstance(n,(int,float)):rs.append(f'pub const DESKTOP_{re.sub(r"([A-Z])",r"_\1",name).upper()}: f32 = {float(n)};')
for name,n in tokens['mobile'].items():
    if isinstance(n,(int,float)):rs.append(f'pub const MOBILE_{re.sub(r"([A-Z])",r"_\1",name).upper()}: f32 = {float(n)};')
rs+=['','/// Stable role, Phosphor name, SVG bytes. White fills support runtime tinting.','pub static ICONS: &[(&str, &str, &[u8])] = &[']
for i in icons:rs.append(f'    ("{i["role"]}", "{i["name"]}", include_bytes!("{i["svg"]}")),')
rs+= ['];','','/// Font family, weight, role, TrueType bytes.','pub static FONTS: &[(&str, u16, &str, &[u8])] = &[']
for f in fonts:rs.append(f'    ("{f["family"]}", {500 if f["weight"]=="Medium" else 400}, "{f["role"]}", include_bytes!("{f["path"]}")),')
rs+= ['];','']
(DEST/'assets.rs').write_text('\n'.join(rs))
# Native integration makes this module part of cargo fmt. Hash the formatted bytes.
subprocess.run(['rustfmt', '--edition', '2024', str(DEST/'assets.rs')], cwd=ROOT, check=True)
manifest={'version':1,'icons':icons,'fonts':fonts,'tokenFile':'tokens.json','themeFile':'themes.json','rustBindings':'assets.rs','figmaUrl':'https://www.figma.com/design/fdGlrRfs8AGQfpmCDhU3Bp','references':{'mobile':'../../design/mobile/screens','mobileOverview':'../../design/mobile/mobile-mockups.png','sidebarSpacing':'../../design/sidebar-spacing/sidebar-comparison.svg','pendingSidebarEdit':'../../design/compact-desktop-sidebar.figma.js'},'files':[]}
for p in sorted(DEST.rglob('*')):
    if p.is_file() and p.name!='manifest.json':
        manifest['files'].append({'path':str(p.relative_to(DEST)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
(DEST/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'icons':len(icons),'fonts':len(fonts),'destination':str(DEST)}))
