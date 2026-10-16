#!/usr/bin/env python3
"""Draft contract checks and example packer, not the production publisher validator.

Uses Python's standard library and xmllint. The production Rust validator,
rights approval, catalog signing, payload compiler, and app installer are future
work. This checker exercises the checked-in examples and isolated SQL model.
"""
import argparse
from copy import deepcopy
import hashlib
import io
from pathlib import Path, PurePosixPath
import re
import sqlite3
import struct
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zipfile

BASE = Path(__file__).resolve().parent
REPO = BASE.parents[2]
EXAMPLE = BASE / 'examples/romans-study'
NS = {'r': 'urn:fontes:resource:1'}
URI = '{urn:fontes:resource:1}'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def xsd(path, schema, valid=True):
    result = subprocess.run(['xmllint', '--nonet', '--noout', '--schema', str(BASE / schema), str(path)], capture_output=True, text=True, check=False)
    require((result.returncode == 0) == valid, f'{path.name}: unexpected XSD result: {result.stderr}')


def read_xml(path):
    data = path.read_bytes()
    require(len(data) <= 16 * 1024 * 1024, 'XML file too large')
    require(b'<!DOCTYPE' not in data and b'<!ENTITY' not in data, 'DTD/entity declaration')
    return ET.fromstring(data.decode('utf-8'))


def relative_path(value):
    path = PurePosixPath(value)
    require(value and not path.is_absolute() and '\\' not in value, 'invalid relative path')
    require(all(p not in ('.', '..', '') for p in value.split('/')), 'unsafe path component')
    return str(path)


def registry():
    import json
    return {b['osis']: b for b in json.loads((REPO / 'data/canonical/registry-v1.json').read_text())['books']}


def reference(value, end=False):
    parts = value.split('.')
    books = registry()
    require(parts[0] in books and 1 <= len(parts) <= 3, f'unknown reference {value}')
    book = books[parts[0]]
    counts = book['verse_counts']
    chapter = int(parts[1]) if len(parts) > 1 else len(counts) if end else 1
    require(1 <= chapter <= len(counts), f'invalid chapter {value}')
    verse = int(parts[2]) if len(parts) > 2 else counts[chapter-1] if end else 1
    require(1 <= verse <= counts[chapter-1], f'invalid verse {value}')
    return book['id'] * 1000000 + chapter * 1000 + verse


def check_graph(graph):
    visiting, visited = set(), set()

    def visit(node):
        require(node not in visiting, 'broader relationship cycle')
        if node in visited:
            return
        visiting.add(node)
        for parent in graph.get(node, []):
            visit(parent)
        visiting.remove(node)
        visited.add(node)
    for node in graph:
        visit(node)


def publisher_concepts(root, publisher):
    prefix = f'urn:fontes:publisher:{publisher}:concept:'
    require(root.get('scope') == 'publisher' and root.get('publisher-id') == publisher, 'wrong proposal owner')
    for concept in root.findall('r:concept', NS):
        require(concept.get('id', '').startswith(prefix), 'publisher cannot define central concepts')


def semantic_example():
    metadata = read_xml(EXAMPLE / 'metadata.xml')
    index = read_xml(EXAMPLE / 'index.xml')
    resource_id = metadata.get('id')
    publisher = metadata.find('r:publisher', NS).get('id')
    require(resource_id.startswith(publisher + '.'), 'resource namespace mismatch')
    require(re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', metadata.get('version')), 'version')
    require(index.get('resource-id') == resource_id, 'index resource mismatch')
    required = {'metadata.xml', 'index.xml', 'rights/LICENSE.txt', 'rights/ATTRIBUTION.txt'}
    cover = metadata.find('r:cover', NS)
    cover_path = relative_path(cover.get('href'))
    require(cover_path in ('assets/cover.png', 'assets/cover.jpg'), 'cover path')
    required.add(cover_path)
    cover_bytes = (EXAMPLE / cover_path).read_bytes()
    require(len(cover_bytes) <= 4 * 1024 * 1024, 'cover size')
    # The checked-in fixture deliberately uses PNG; JPEG decoding is a production
    # validator requirement, not an implemented feature of this example checker.
    require(cover_bytes[:8] == b'\x89PNG\r\n\x1a\n' and cover.get('media-type') == 'image/png', 'example PNG type')
    require(struct.unpack('>II', cover_bytes[16:24]) == (1200, 1800), 'cover dimensions')
    for name in ('rights/LICENSE.txt', 'rights/ATTRIBUTION.txt'):
        require((EXAMPLE / name).read_text().strip(), f'empty {name}')
    proposals_path = EXAMPLE / 'taxonomy/concepts.xml'
    proposals = read_xml(proposals_path)
    publisher_concepts(proposals, publisher)
    required.add('taxonomy/concepts.xml')
    concepts = {}
    for root in (read_xml(BASE / 'taxonomy.example.xml'), proposals):
        for concept in root.findall('r:concept', NS):
            identity = concept.get('id')
            require(identity not in concepts, 'duplicate concept')
            concepts[identity] = concept
            labels = concept.findall('r:label', NS)
            preferred = [x.get('language') for x in labels if x.get('kind') == 'preferred']
            require(preferred and len(preferred) == len(set(preferred)), 'preferred label ambiguity')
    graph = {}
    for identity, concept in concepts.items():
        graph[identity] = []
        for link in concept:
            if link.tag in (URI + 'broader', URI + 'related'):
                require(link.get('id') in concepts, 'unknown concept target')
                if link.tag == URI + 'broader':
                    graph[identity].append(link.get('id'))
    check_graph(graph)
    identities = {s.get('id') for s in index.findall('.//r:section', NS)}
    documents, blocks = 0, 0
    for entry in index.findall('.//r:document', NS):
        path = relative_path(entry.get('href'))
        require(path.startswith('content/') and path.endswith('.xml') and path not in required, 'invalid content path')
        required.add(path)
        doc = read_xml(EXAMPLE / path)
        xsd(EXAMPLE / path, 'content.xsd')
        require(doc.get('resource-id') == resource_id and doc.get('id') == entry.get('id'), 'document identity mismatch')
        for node in [doc] + doc.findall('r:block', NS):
            identity = node.get('id')
            require(identity not in identities, 'duplicate global node ID')
            identities.add(identity)
            if node is not doc:
                blocks += 1
                text = node.find('r:text', NS)
                if node.get('status', 'present') == 'omitted':
                    require(node.get('kind') == 'verse' and text is None, 'omitted verse has text or wrong type')
                else:
                    require(text is not None and (''.join(text.itertext()).strip() or text.find('.//r:image', NS) is not None), 'missing block content')
                if node.get('kind') == 'verse':
                    require(node.get('source-ref') and node.find("r:annotations/r:scripture[@relation='text-of']", NS) is not None, 'verse lacks source/mapping')
        for node in doc.iter():
            if node.tag in (URI + 'scripture', URI + 'ref'):
                require(reference(node.get('start')) <= reference(node.get('end', node.get('start')), True), 'reversed scripture range')
            if node.tag in (URI + 'concept', URI + 'term'):
                require(node.get('id', node.get('concept')) in concepts, 'undeclared concept')
            if node.tag == URI + 'strongs':
                code = node.get('code')
                number = int(code[1:])
                require(0 < number <= (5624 if code[0] == 'G' else 8674) and code == f'{code[0]}{number:04}', 'Strong\'s code')
            if node.tag == URI + 'image':
                media = relative_path(node.get('src'))
                require(media.startswith('media/'), 'image outside media')
                required.add(media)
        documents += 1
    for root in (metadata, *concepts.values()):
        for node in root.iter():
            if node.tag in (URI + 'range', URI + 'evidence'):
                require(reference(node.get('start')) <= reference(node.get('end', node.get('start')), True), 'invalid coverage/evidence')
    files = set()
    folded = set()
    for path in EXAMPLE.rglob('*'):
        require(not path.is_symlink(), 'source symlink')
        if path.is_file():
            relative = path.relative_to(EXAMPLE).as_posix()
            require(relative.casefold() not in folded, 'case-fold collision')
            files.add(relative)
            folded.add(relative.casefold())
    require(files == required, f'unlisted/missing example files: {files ^ required}')
    return documents, blocks


def negative_checks():
    with tempfile.TemporaryDirectory() as temp:
        target = Path(temp) / 'invalid.xml'
        for filename, schema, mutate in [
            ('metadata.xml', 'metadata.xsd', lambda root: root.find('r:cover', NS).set('width', '900')),
            ('content/rom/001.xml', 'content.xsd', lambda root: root.append(ET.Element(URI + 'script'))),
            ('content/rom/001.xml', 'content.xsd', lambda root: root.append(deepcopy(root.find('r:block', NS)))),
            ('index.xml', 'index.xsd', lambda root: root.append(deepcopy(root.find('.//r:document', NS)))),
        ]:
            root = read_xml(EXAMPLE / filename)
            mutate(root)
            ET.ElementTree(root).write(target, encoding='utf-8', xml_declaration=True)
            xsd(target, schema, valid=False)
    for action in [lambda: reference('Rom.1.99'), lambda: relative_path('content/../rights/LICENSE.txt'), lambda: check_graph({'a':['b'], 'b':['a']})]:
        try:
            action()
        except ValueError:
            pass
        else:
            raise AssertionError('invalid semantic fixture accepted')
    root = read_xml(EXAMPLE / 'taxonomy/concepts.xml')
    root.find('r:concept', NS).set('id', 'urn:fontes:concept:hijacked')
    try:
        publisher_concepts(root, 'org.example')
    except ValueError:
        pass
    else:
        raise AssertionError('central namespace overwrite accepted')


def sql_model_checks():
    active = sqlite3.connect(':memory:')
    active.executescript((BASE / 'library-model.sql').read_text())
    verse = reference('Rom.1.3')
    active.execute('INSERT INTO canonical_refs VALUES(?,?,1,3)', (verse, 'Rom'))
    active.execute("INSERT INTO strongs_codes VALUES('G5207')")
    central, provisional = 'urn:fontes:concept:incarnation', 'urn:fontes:publisher:org.example:concept:local-topic'
    for concept in (central, provisional):
        active.execute('INSERT INTO concepts VALUES(?,?,?)', (concept,'example','Example scope'))
    for name, concept in [('org.example.one', central), ('org.example.two', provisional)]:
        active.execute('INSERT INTO packages VALUES(?,?,?,1,?)', (name,'org.example','1.0.0','0'*64))
        active.execute("INSERT INTO content_nodes VALUES(?, 'p1', NULL, 'paragraph', 0, 'Messiah study')", (name,))
        active.execute("INSERT INTO scripture_links VALUES(?,'p1',?,?,'alludes-to','author','source package')", (name,verse,verse))
        active.execute("INSERT INTO concept_assertions VALUES(?,'p1',?,'discusses','source package')", (name,concept))
        active.execute("INSERT INTO search_documents(package_id,node_id,text) VALUES(?,'p1','Messiah study')", (name,))
    active.execute("INSERT INTO search_fts(search_fts) VALUES('rebuild')")
    active.commit()
    count = lambda db: db.execute('SELECT count(*) FROM scripture_links WHERE start_ref_id<=? AND end_ref_id>=?', (verse,verse)).fetchone()[0]
    require(count(active)==2, 'shared context lookup missed a package')
    active.execute('INSERT INTO concept_resolutions VALUES(?,?,?,?)', (provisional,central,'taxonomy-2','review-1'))
    require(active.execute('SELECT count(*) FROM effective_concept_assertions WHERE effective_concept_id=?', (central,)).fetchone()[0]==2, 'concept mapping did not reach old packages')
    require(active.execute('SELECT submitted_concept_id FROM concept_assertions WHERE package_id=?', ('org.example.two',)).fetchone()[0]==provisional, 'mapping rewrote author assertion')
    active.commit()
    user = sqlite3.connect(':memory:')
    user.executescript((REPO / 'data/schema/user-v1.sql').read_text())
    user.execute('INSERT INTO canonical_refs VALUES(?,?,1,3)', (verse,'Rom'))
    user.execute("INSERT INTO reference_ranges VALUES('range',?,?)", (verse,verse))
    user.execute("INSERT INTO annotations(id,range_id,type,content,created_at,updated_at) VALUES('note','range','note','Keep this note','2026-10-16','2026-10-16')")
    user.commit()
    candidate = sqlite3.connect(':memory:')
    active.backup(candidate)
    candidate.execute('PRAGMA foreign_keys=ON')
    try:
        with candidate:
            candidate.execute("INSERT INTO scripture_links VALUES('org.example.one','p1',999,999,'cites','author','invalid')")
    except sqlite3.IntegrityError:
        pass
    else:
        raise AssertionError('dangling reference accepted')
    require(count(candidate)==2 and count(active)==2, 'failed transaction damaged content')
    with candidate:
        candidate.execute("DELETE FROM packages WHERE resource_id='org.example.one'")
        candidate.execute("INSERT INTO search_fts(search_fts) VALUES('rebuild')")
    require(count(candidate)==1 and count(active)==2, 'staged uninstall damaged other package or old generation')
    require(candidate.execute("SELECT count(*) FROM search_fts WHERE search_fts MATCH 'Messiah'").fetchone()[0]==1, 'uninstall left search postings')
    require(candidate.execute('PRAGMA foreign_key_check').fetchall()==[], 'uninstall left broken FKs')
    require(candidate.execute('SELECT count(*) FROM concepts').fetchone()[0]==2, 'uninstall deleted shared concepts')
    require(user.execute('SELECT content FROM annotations').fetchone()[0]=='Keep this note', 'user note was lost')
    active.close()
    candidate.close()
    user.close()


def json_checks():
    import json
    from jsonschema import Draft202012Validator, FormatChecker
    schemas = {}
    for name in ['catalog', 'delivery-manifest']:
        schema = json.loads((BASE / f'{name}.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        sample = json.loads((BASE / f'{name}.example.json').read_text())
        validator.validate(sample)
        schemas[name] = (validator, sample)
    validator, sample = schemas['catalog']
    identities = [r['resource_id'] for r in sample['resources']]
    require(len(identities) == len(set(identities)), 'duplicate catalog resource')
    for bundle in sample['bundles']:
        require(set(bundle['resource_ids']) <= set(identities), 'unknown bundle member')
    bad = deepcopy(sample)
    bad['resources'][0]['cover']['thumbnail']['width'] = 900
    require(list(validator.iter_errors(bad)), 'wrong catalog cover size accepted')
    bad = deepcopy(sample)
    bad['resources'][0]['rights']['permissions']['offline_display'] = 'deny'
    require(list(validator.iter_errors(bad)), 'unreadable resource in free catalog accepted')
    bad = deepcopy(sample)
    bad['resources'][0]['artifact']['sha256'] = 'not a digest'
    require(list(validator.iter_errors(bad)), 'invalid catalog digest accepted')
    validator, sample = schemas['delivery-manifest']
    bad = deepcopy(sample)
    bad['package_protocol_version'] = 99
    require(list(validator.iter_errors(bad)), 'unknown package protocol accepted')
    bad = deepcopy(sample)
    bad['surprise_sql'] = 'DROP TABLE packages'
    require(list(validator.iter_errors(bad)), 'unknown delivery field accepted')
    source_samples = {
        'submission': REPO / 'library/submissions/org.example.romans-study/submission.json',
        'publication-policy': REPO / 'library/policy.json',
        'approval': REPO / 'library/approvals/org.example.romans-study--1.0.0.json',
    }
    for name, path in source_samples.items():
        schema = json.loads((BASE / f'{name}.schema.json').read_text())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        sample = json.loads(path.read_text())
        validator.validate(sample)
        bad = deepcopy(sample)
        bad['unexpected_command'] = 'execute me'
        require(list(validator.iter_errors(bad)), f'unknown {name} field accepted')
    schema = json.loads((BASE / 'source-release-manifest.schema.json').read_text())
    Draft202012Validator.check_schema(schema)


def pack_example(output):
    stream = io.BytesIO()
    with zipfile.ZipFile(stream, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in sorted(p for p in EXAMPLE.rglob('*') if p.is_file()):
            info = zipfile.ZipInfo(path.relative_to(EXAMPLE).as_posix(), date_time=(1980,1,1,0,0,0))
            info.create_system=3
            info.external_attr=0o100644 << 16
            info.compress_type=zipfile.ZIP_DEFLATED
            archive.writestr(info, path.read_bytes(), compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    data = stream.getvalue()
    if output:
        output.mkdir(parents=True, exist_ok=True)
        target = output / 'org.example.romans-study--1.0.0.fontes-source.zip'
        if target.exists():
            require(target.read_bytes()==data, 'refusing to overwrite a different example archive')
        else:
            target.write_bytes(data)
        print(f'Example archive: {target} ({len(data)} bytes, sha256 {hashlib.sha256(data).hexdigest()})')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pack', type=Path, metavar='OUTPUT_DIRECTORY')
    parser.add_argument('--json', action='store_true', help='Also check JSON contracts (requires jsonschema 4.26.0)')
    options = parser.parse_args()
    for path, schema in [('metadata.xml','metadata.xsd'), ('index.xml','index.xsd'), ('taxonomy/concepts.xml','concepts.xsd')]:
        xsd(EXAMPLE / path, schema)
    xsd(BASE / 'taxonomy.example.xml', 'concepts.xsd')
    documents, blocks = semantic_example()
    negative_checks()
    sql_model_checks()
    if options.json:
        json_checks()
    require(pack_example(options.pack)==pack_example(None), 'example packaging is not deterministic')
    print(f'Draft contracts checked: {documents} documents, {blocks} blocks; invalid structures/references rejected; shared SQL context, concept mapping, rollback, uninstall, search and user-note preservation passed.')


if __name__ == '__main__':
    main()
