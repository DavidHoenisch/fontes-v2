# Bundled book openings

66 original Covenant Forms SVG illustrations, mapped by permanent canonical book ID (1–66). Source compositions and generator: design/book-covers/. The art is Fontes-authored; repository licensing applies.

introductions.json contains public-domain historical text from Matthew Henry's Commentary, transcribed by CCEL. Every record links its separate Introduction section, original volume XML URL and SHA-256. Acts–Revelation credit Henry and continuators; supplied “Completed by” lines are retained. Historical interpretations remain attributed to their authors.

Extraction selects direct paragraph children of the Introduction div2 only, excluding centered title typography and all chapter commentary. Genesis and Matthew retain the final book-specific paragraph, excluding general Bible/testament discussion. Other books retain the complete introduction prose, including relevant general context. No modern editorial summaries substitute for Henry's text. Whitespace is normalized; text is otherwise retained. Source pages are linked from the app.

Rebuild with python3 design/book-covers/bundle_app.py (network required; source XML cached under the system temporary directory). Review changed source hashes and introduction boundaries before accepting regenerated text. No runtime download, app command, SQL contract or signed library mutation. Both Reading and Study profiles can use these bundled assets.
