#!/usr/bin/env python3
"""Build the selected Wellspring SVG masters. Run: uv run --with fonttools==4.67.0 python tools/brand-assets.py"""
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
import json
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/brand/wellspring'
OUT.mkdir(parents=True, exist_ok=True)
TEAL, IVORY, CLAY = '#233F47', '#F7F0E0', '#AC563D'
# Geometry is deliberately sparse; all colored masters share the same paths.
ARCH = 'M48 196V112a80 80 0 0 1 160 0v84c-3-12-7-20-12-24v-60a68 68 0 0 0-136 0v60c-5 4-9 12-12 24Z'
CENTER = 'M126 185c-1-29-13-55-13-73 0-16 7-27 15-27s15 11 15 27c0 18-12 44-13 73Z'
SIDE = 'M120 185c-4-26-14-43-34-50-13-5-19-13-14-22 5-10 18-8 28 1 18 17 23 42 24 71Z'
BASIN = 'M69 181h118q3 0 3 4v5q0 4-5 5-9 3 0 7h11v10H60v-10h11q9-4 0-7-5-1-5-5v-5q0-4 3-4Z'
def symbol(ink=IVORY, base=CLAY):
    return f'<path fill="{ink}" d="{ARCH}"/><path fill="{ink}" d="{CENTER}"/><path fill="{ink}" d="{SIDE}"/><path fill="{ink}" transform="translate(256 0) scale(-1 1)" d="{SIDE}"/><path fill="{base}" d="{BASIN}"/>'
def svg(body, box='0 0 256 256', title='Fontes Wellspring'):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{box}" role="img" aria-label="{title}"><title>{title}</title>{body}</svg>\n'
files = {
 'symbol.svg': svg(symbol()),
 'symbol-ink.svg': svg(symbol('currentColor','currentColor')),
 'symbol-reversed.svg': svg(symbol(IVORY,IVORY)),
 'app-icon.svg': svg(f'<rect width="256" height="256" fill="{TEAL}"/>'+symbol()),
 'app-icon-rounded.svg': svg(f'<rect width="256" height="256" rx="56" fill="{TEAL}"/>'+symbol()),
 'android-background.svg': svg(f'<rect width="256" height="256" fill="{TEAL}"/>'),
 'android-foreground.svg': svg('<g transform="translate(38.4 38.4) scale(.7)">'+symbol()+'</g>'),
 'android-monochrome.svg': svg('<g transform="translate(38.4 38.4) scale(.7)">'+symbol('#FFFFFF','#FFFFFF')+'</g>'),
}
# Outline the existing licensed literary typeface so exported wordmarks need no font download.
font = TTFont(ROOT/'assets/ui/fonts/CormorantGaramond-Italic.ttf')
glyphs = font.getGlyphSet(); cmap = font.getBestCmap(); scale = 112/font['head'].unitsPerEm
x = 0; outlines=[]
for letter in 'fontes':
    name = cmap[ord(letter)]; pen = SVGPathPen(glyphs)
    glyphs[name].draw(TransformPen(pen,(scale,0,0,-scale,x,172)))
    outlines.append(f'<path d="{pen.getCommands()}"/>')
    x += font['hmtx'][name][0]*scale
wordmark=''.join(outlines)
files['wordmark.svg']=svg(f'<g fill="currentColor" transform="translate(28 0)">{wordmark}</g>',f'0 0 {int(x+60)} 224','Fontes wordmark')
files['logo-horizontal.svg']=svg(symbol(TEAL,CLAY)+f'<g fill="{TEAL}" transform="translate(280 0)">{wordmark}</g>',f'0 0 {int(x+310)} 256','Fontes Wellspring logo')
files['logo-horizontal-ink.svg']=svg(symbol('currentColor','currentColor')+f'<g fill="currentColor" transform="translate(280 0)">{wordmark}</g>',f'0 0 {int(x+310)} 256','Fontes Wellspring monochrome logo')
for name, content in files.items(): (OUT/name).write_text(content)
(OUT/'tauri-icons.json').write_text(json.dumps({'default':'app-icon-rounded.svg','bg_color':TEAL,'android_bg':'android-background.svg','android_fg':'android-foreground.svg','android_fg_scale':100,'android_monochrome':'android-monochrome.svg'},indent=2)+'\n')
# Preserve Android's existing drawable names as vector versions of the same mark.
res=ROOT/'apps/fontes-web/src-tauri/gen/android/app/src/main/res'
paths=''.join(f'<path android:fillColor="{IVORY}" android:pathData="{path}"/>' for path in [ARCH,CENTER,SIDE])
paths+=f'<group android:pivotX="128" android:scaleX="-1"><path android:fillColor="{IVORY}" android:pathData="{SIDE}"/></group><path android:fillColor="{CLAY}" android:pathData="{BASIN}"/>'
(res/'drawable-v24/ic_launcher_foreground.xml').write_text(f'<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="256" android:viewportHeight="256">{paths}</vector>\n')
(res/'drawable/ic_launcher_background.xml').write_text(f'<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="256" android:viewportHeight="256"><path android:fillColor="{TEAL}" android:pathData="M0,0h256v256h-256Z"/></vector>\n')
(ROOT/'apps/fontes-web/public/fontes-icon.svg').write_text(files['app-icon-rounded.svg'])
import hashlib
(OUT/'brand.json').write_text(json.dumps({
    'id':'fontes-wellspring-v1','selected':'2026-10-17','palette':{'teal':TEAL,'ivory':IVORY,'clay':CLAY},
    'wordmark_font':'Cormorant Garamond Italic','font_license':'SIL Open Font License 1.1',
    'assets':{name:{'sha256':hashlib.sha256(content.encode()).hexdigest(),'bytes':len(content.encode())} for name,content in files.items()}
},indent=2)+'\n')
