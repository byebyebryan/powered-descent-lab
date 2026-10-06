#!/usr/bin/env node
// Read-only local Markdown links/anchors. Generated evidence is not a Git input.
import {execFileSync} from 'node:child_process';
import {existsSync, readFileSync} from 'node:fs';
import {dirname, isAbsolute, relative, resolve, sep} from 'node:path';
import {fileURLToPath} from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');

export function discoverMarkdownFiles(root = ROOT) {
  return [...new Set(execFileSync('rtk', [
    'proxy', 'git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z',
  ], {cwd: root, encoding: 'utf8'}).split('\0')
    .filter(file => file.endsWith('.md') && existsSync(resolve(root, file))))].sort();
}

function withoutFences(source) {
  let fence = null;
  return source.split('\n').map(line => {
    const marker = /^\s{0,3}(`{3,}|~{3,})(.*)$/.exec(line);
    if (fence) {
      if (marker && marker[1][0] === fence[0]
        && marker[1].length >= fence.length && !marker[2].trim()) fence = null;
      return '';
    }
    if (marker) { fence = marker[1]; return ''; }
    return line;
  }).join('\n');
}

const blank = text => text.replace(/[^\n]/g, ' ');

function proseText(source, preserveInlineCode = false) {
  // Process code spans and comments in source order: a literal comment opener
  // in code must not consume later prose, nor may comment text open a code span.
  return withoutFences(source).replace(
    /(?<!`)(`+)(?!`)[\s\S]*?(?<!`)\1(?!`)|<!--[\s\S]*?(?:-->|$)/g,
    (match, codeDelimiter) => codeDelimiter && preserveInlineCode ? match : blank(match),
  );
}

export function markdownAnchors(source) {
  const text = proseText(source, true);
  const anchors = new Set();
  const headingIds = new Set();
  for (const match of text.matchAll(/^\s{0,3}#{1,6}\s+(.+)$/gm)) {
    const base = match[1].replace(/\s+#+\s*$/, '').replace(/<[^>]*>/g, '')
      .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1').toLowerCase()
      .replace(/[^\p{L}\p{N}_\-\s]/gu, '').replace(/\s/g, '-');
    let id = base;
    for (let suffix = 1; headingIds.has(id); suffix += 1) id = `${base}-${suffix}`;
    headingIds.add(id);
    anchors.add(id);
  }
  for (const match of proseText(source).matchAll(/<[^>]+\bid\s*=\s*["']([^"']+)["'][^>]*>/g)) {
    anchors.add(match[1]);
  }
  return anchors;
}

export function checkDocumentationLinks(root = ROOT, files = discoverMarkdownFiles(root)) {
  root = resolve(root);
  const issues = [];
  let links = 0;
  let generatedEvidenceLinks = 0;
  const anchorCache = new Map();
  for (const file of files) {
    // Literal examples are not links. Retain newlines for source diagnostics;
    // heading extraction separately keeps inline-code text for its slug.
    const text = proseText(readFileSync(resolve(root, file), 'utf8'));
    for (const match of text.matchAll(/!?\[[^\]\n]*\]\((<[^>]+>|[^\s)]+)(?:\s+[^)]*)?\)/g)) {
      const href = match[1].replace(/^<|>$/g, '');
      if (/^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith('//')) continue;
      const line = text.slice(0, match.index).split('\n').length;
      const issue = message => issues.push({file, line, href, message});
      let path, fragment;
      try {
        const split = href.indexOf('#');
        path = decodeURIComponent((split < 0 ? href : href.slice(0, split)).split('?')[0]);
        fragment = split < 0 ? '' : decodeURIComponent(href.slice(split + 1));
      } catch { issue('invalid URL encoding'); continue; }
      const target = path ? resolve(root, dirname(file), path) : resolve(root, file);
      const inside = relative(root, target);
      if (inside === '..' || inside.startsWith(`..${sep}`) || isAbsolute(inside)) {
        issue('link escapes the repository'); continue;
      }
      if (inside === 'outputs' || inside.startsWith(`outputs${sep}`)) {
        generatedEvidenceLinks += 1;
        continue;
      }
      links += 1;
      if (!existsSync(target)) { issue('missing repository target'); continue; }
      if (fragment && target.endsWith('.md')) {
        if (!anchorCache.has(target)) anchorCache.set(target, markdownAnchors(readFileSync(target, 'utf8')));
        if (!anchorCache.get(target).has(fragment)) issue(`missing Markdown anchor #${fragment}`);
      }
    }
  }
  return {files: files.length, links, generatedEvidenceLinks, issues};
}

export function main(args = process.argv.slice(2)) {
  if (args.length) {
    if (args.length === 1 && args[0] === '--help') {
      process.stdout.write('Usage: node scripts/check-docs.mjs\nRead-only local links and Markdown anchors; generated outputs and external URLs are skipped.\n');
      return 0;
    }
    throw new Error('check-docs accepts no arguments other than --help');
  }
  const result = checkDocumentationLinks();
  for (const issue of result.issues) {
    process.stderr.write(`${issue.file}:${issue.line}: ${issue.message}: ${issue.href}\n`);
  }
  process.stdout.write(`Documentation: ${result.files} files, ${result.links} local links, ${result.generatedEvidenceLinks} generated-evidence links skipped, ${result.issues.length} issues.\n`);
  return result.issues.length ? 1 : 0;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { process.exitCode = main(); }
  catch (error) { process.stderr.write(`${error.message}\n`); process.exitCode = 1; }
}
