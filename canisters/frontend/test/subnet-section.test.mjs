import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, access } from 'node:fs/promises';
const publicFile = name => readFile(new URL(`../public/${name}`, import.meta.url), 'utf8');
const embed = 'https://dashboard.internetcomputer.org/embed/subnet-map?subnet=pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae';
test('one accessible subnet section immediately precedes the unchanged manifesto inside main', async () => {
  const html = await publicFile('index.html');
  assert.match(html, /<\/header>\s*<main id="main-content" tabindex="-1">\s*<section id="subnet-map"/);
  assert.equal(html.match(/id="subnet-map"/g).length, 1);
  assert.match(html, /class="subnet-section" aria-labelledby="subnet-title"/);
  assert.match(html, /<h2 id="subnet-title">Explore the subnet hosting Event Horizon\.<\/h2>/);
  assert.match(html, /<\/section>\s*<section class="manifesto" aria-labelledby="manifesto-title">/);
  assert.match(html, /A cost-effective path for ledger-aware canisters\./);
  assert.match(html, /<section class="section" id="network"/);
});
test('one real lazy iframe retains the reviewed security boundary and permanent source links', async () => {
  const html = await publicFile('index.html');
  const frames = html.match(/<iframe\b[^>]*>/g);
  assert.equal(frames.length, 1);
  for (const attribute of [`src="${embed}"`, 'loading="lazy"', 'sandbox="allow-scripts allow-same-origin"', 'referrerpolicy="no-referrer"']) assert.ok(frames[0].includes(attribute), attribute);
  assert.match(frames[0], /title="ICP Dashboard subnet placement map[^\"]+"/);
  assert.ok(html.includes(embed.replace('/embed/subnet-map?subnet=', '/network/subnet/')));
  assert.match(html, /methodology#embeddable-subnet-map/);
  assert.match(html, /If the map does not load/);
  assert.match(html, /Subnet map provided by ICP Dashboard/);
});
test('built entry and styles contain no widget controller or legacy controls', async () => {
  for (const name of ['index.html', 'app.js', 'app.bundle.js', 'styles.css']) {
    const source = await publicFile(name);
    assert.doesNotMatch(source, /subnet-widget|subnet-dialog|subnet-map-open|initSubnetWidget|subnet-(?:expand|minimise|dismiss|close)/, name);
  }
  const html = await publicFile('index.html');
  assert.doesNotMatch(html, /<dialog|<aside/);
  await assert.rejects(access(new URL('../public/subnet-widget.js', import.meta.url)));
});
