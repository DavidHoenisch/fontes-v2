"""Bundle the visual prototypes with their local fonts, icons and licenses."""
from pathlib import Path
import re
import zipfile

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'design/color-studies'
ASSETS = ROOT / 'assets/ui'
OUTPUT = ROOT / 'dist/fontes-color-studies.zip'
css = (SOURCE / 'styles.css').read_text().replace('../../assets/ui/fonts/', 'assets/fonts/')
js = (SOURCE / 'app.js').read_text().replace('../../assets/ui/icons/phosphor/regular/', 'assets/icons/')
icons = set(re.findall(r"\['(?:Read|Search|Notes|Study)','([^']+)'\]", js))
icons |= set(re.findall(r"icon\('([^']+)'\)", js))
OUTPUT.parent.mkdir(exist_ok=True)
with zipfile.ZipFile(OUTPUT, 'w', zipfile.ZIP_DEFLATED) as bundle:
    for path in sorted(SOURCE.glob('*.html')):
        bundle.write(path, path.name)
    bundle.writestr('styles.css', css)
    bundle.writestr('app.js', js)
    for name in ['Lora-Regular.ttf', 'IBMPlexSans-Regular.ttf', 'IBMPlexSans-Medium.ttf', 'NotoSerifHebrew-Regular.ttf']:
        bundle.write(ASSETS / 'fonts' / name, 'assets/fonts/' + name)
    for name in sorted(icons):
        path = ASSETS / 'icons/phosphor/regular' / f'{name}.svg'
        bundle.write(path, 'assets/icons/' + path.name)
    for path in sorted((ASSETS / 'licenses').rglob('*')):
        if path.is_file():
            bundle.write(path, 'licenses/' + str(path.relative_to(ASSETS / 'licenses')))
    bundle.writestr('README.txt', 'Open index.html. Seven interactive color directions, each showing reading, word study and selection actions. All prototype state is temporary. No remote dependencies. Font and icon licenses are included.\n')
print(OUTPUT)
