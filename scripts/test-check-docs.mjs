import test from 'node:test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdirSync, mkdtempSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {checkDocumentationLinks, discoverMarkdownFiles, markdownAnchors} from './check-docs.mjs';

function fixture(run) {
  const root = mkdtempSync(join(tmpdir(), 'pd-doc-links-'));
  try {
    mkdirSync(join(root, 'docs'));
    return run(root, (file, text) => writeFileSync(join(root, file), text));
  } finally { rmSync(root, {recursive: true, force: true}); }
}

test('local links resolve from the document, including encoded names and explicit anchors', () => {
  fixture((root, write) => {
    write('README.md', '[Guide](docs/a%20guide.md#use-it)\n[Named](docs/a%20guide.md#kept-anchor)');
    write('docs/a guide.md', '# Use `it`\n<a id="kept-anchor"></a>\n[Home](../README.md)');
    const result = checkDocumentationLinks(root, ['README.md', 'docs/a guide.md']);
    assert.deepEqual(result.issues, []);
    assert.equal(result.links, 3);
  });
});

test('heading ids preserve duplicate suffixes and ignore fenced headings', () => {
  assert.deepEqual([...markdownAnchors('# Same\n# Same\n# Same-1\n~~~md\n# Hidden\n~~~\n# A & B')],
    ['same', 'same-1', 'same-1-1', 'a--b']);
});

test('missing targets and anchors report the original line, not an abbreviated document', () => {
  fixture((root, write) => {
    write('README.md', '```md\n[Example](missing.md)\n```\n[Missing](gone.md)\n[Anchor](docs/guide.md#absent)');
    write('docs/guide.md', '# Present');
    const result = checkDocumentationLinks(root, ['README.md']);
    assert.equal(result.issues.length, 2);
    assert.deepEqual(result.issues.map(issue => issue.line), [4, 5]);
    assert.match(result.issues[0].message, /missing repository target/);
    assert.match(result.issues[1].message, /missing Markdown anchor/);
  });
});

test('fresh checkouts need neither external pages nor ignored generated evidence', () => {
  fixture((root, write) => {
    write('docs/guide.md', '[Archive](https://example.invalid/blob/revision/source.rs)\n'
      + '[Receipt](../outputs/retained/receipt.json)\n[Code](#current)\n## Current');
    const result = checkDocumentationLinks(root, ['docs/guide.md']);
    assert.deepEqual(result.issues, []);
    assert.equal(result.generatedEvidenceLinks, 1);
    assert.equal(result.links, 1);
  });
});

test('malformed encoding and escaping paths fail instead of reading outside the checkout', () => {
  fixture((root, write) => {
    write('README.md', '[Broken](bad%ZZ.md)\n[Outside](../outside.md)');
    const result = checkDocumentationLinks(root, ['README.md']);
    assert.equal(result.issues.length, 2);
    assert.match(result.issues[0].message, /encoding/);
    assert.match(result.issues[1].message, /escapes/);
  });
});

test('inline code and comments cannot invent links or anchors, while diagnostics retain lines', () => {
  fixture((root, write) => {
    write('README.md', '`[Literal](missing.md)`\n``[Literal](also-missing.md) `tick` ``\n'
      + '<!--\n[Hidden](gone.md)\n## Hidden\n<a id="hidden"></a>\n-->\n'
      + '[Real](docs/guide.md#use-code)\n[Broken](docs/guide.md#absent)');
    write('docs/guide.md', '# Use `code`');
    const result = checkDocumentationLinks(root, ['README.md']);
    assert.equal(result.links, 2);
    assert.equal(result.issues.length, 1);
    assert.equal(result.issues[0].line, 9);
    assert.deepEqual([...markdownAnchors('<!--\n# Hidden\n<a id="hidden"></a>\n-->\n# Shown')], ['shown']);
  });
});

test('Git discovery includes new docs, skips ignored/deleted files and still checks deleted targets', () => {
  fixture((root, write) => {
    execFileSync('rtk', ['proxy', 'git', 'init', '--quiet'], {cwd: root, stdio: 'pipe'});
    write('.gitignore', '/outputs/\n');
    write('README.md', '[Deleted](deleted.md)\n[New](docs/new.md)');
    write('deleted.md', '# Deleted');
    execFileSync('rtk', ['proxy', 'git', 'add', 'README.md', 'deleted.md'], {cwd: root, stdio: 'pipe'});
    rmSync(join(root, 'deleted.md'));
    write('docs/new.md', '# New');
    mkdirSync(join(root, 'outputs'));
    write('outputs/ignored.md', '[Missing](gone.md)');
    assert.deepEqual(discoverMarkdownFiles(root), ['README.md', 'docs/new.md']);
    const result = checkDocumentationLinks(root);
    assert.equal(result.files, 2);
    assert.equal(result.issues.length, 1);
    assert.equal(result.issues[0].href, 'deleted.md');
    assert.match(result.issues[0].message, /missing repository target/);
  });
});

test('literal comment markers cannot hide later links or invent explicit HTML anchors', () => {
  fixture((root, write) => {
    write('README.md', '`<!--`\n[Missing](missing.md)\n<!-- `literal\n'
      + '[Hidden](also-missing.md)\n-->\n[Present](docs/guide.md#shown)');
    write('docs/guide.md', '# Shown');
    const result = checkDocumentationLinks(root, ['README.md']);
    assert.equal(result.links, 2);
    assert.equal(result.issues.length, 1);
    assert.equal(result.issues[0].line, 2);
    assert.equal(result.issues[0].href, 'missing.md');
    assert.deepEqual([...markdownAnchors('# Shown\n`<!--`\n# After\n`<a id="literal"></a>`')],
      ['shown', 'after']);
  });
});
