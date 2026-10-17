import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';

const publicFile = name => new URL(`../public/${name}`, import.meta.url);
const html = await readFile(publicFile('index.html'), 'utf8');
const expected = [
  ['open-source','Open Source & Verifiable','/github-mark-white.svg'],
  ['immutability','Designed for Immutability','/favicon.ico?v=1'],
  ['endowment-funding','Productive Endowment Funding','/jupiter-faucet.ico'],
  ['internet-computer','Powered by Internet Computer Protocol','/icp-infinity.svg'],
];
const nav = html.match(/<nav class="hero__facts"[^>]*>([\s\S]*?)<\/nav>/)?.[1];

test('four hero fact links retain their order, labels, local icons and native fragment destinations', () => {
  assert.ok(nav); assert.match(html, /<nav class="hero__facts" aria-label="Explore Event Horizon">/);
  const anchors = [...nav.matchAll(/<a href="([^"]+)"([^>]*)>([\s\S]*?)<\/a>/g)];
  assert.equal(anchors.length, 4);
  assert.deepEqual(anchors.map(([,href,attrs,content]) => {
    assert.equal(attrs, ''); // No script handlers, overriding names, or new-tab navigation.
    assert.match(content, /<img [^>]*alt=""/);
    assert.doesNotMatch(content, /\b(?:title|aria-label|onclick|onload)=/);
    return [href.slice(1), content.match(/<span>(.*?)<\/span>/)[1].replaceAll('&amp;','&'), content.match(/src="([^"]+)"/)[1]];
  }), expected);
  assert.ok(anchors.every(([,href]) => href.startsWith('#')));
});

test('every hero destination is a unique labelled section after Principles and before additional instances', () => {
  const main = html.match(/<main\b[^>]*>([\s\S]*?)<\/main>/)[1];
  const order = expected.map(([id,label]) => {
    assert.equal([...html.matchAll(new RegExp(`id="${id}"`, 'g'))].length, 1);
    const section = main.match(new RegExp(`<section class="project-detail" id="${id}" aria-labelledby="([^"]+)" tabindex="-1">([\\s\\S]*?)<\\/section>`));
    assert.ok(section, id);
    const [,headingId,content] = section;
    assert.equal([...html.matchAll(new RegExp(`id="${headingId}"`, 'g'))].length, 1);
    assert.ok(content.includes(`<h2 id="${headingId}">${label.replaceAll('&','&amp;')}</h2>`));
    assert.ok(content.includes('class="project-detail__links"'));
    const links = [...content.matchAll(/<a href="([^"]+)"/g)];
    assert.ok(links.length > 0); assert.ok(links.every(([,href]) => href.startsWith('https://')));
    return main.indexOf(`id="${id}"`);
  });
  assert.ok(order[0] > main.indexOf('id="protocol"'));
  assert.deepEqual(order, [...order].sort((a,b)=>a-b));
  assert.ok(order.at(-1) < main.indexOf('class="instances-note"'));
});

test('brand icons preserve the downloaded bytes, use local certified media, and contain no active/remote SVG content', async () => {
  for (const [name,hash] of [
    ['github-mark-white.svg','ccd84c89b1056345608fc3489357f8acc7397e49a3cdc2d418b6c8016911d47b'],
    ['jupiter-faucet.ico','37195f946930cfa8349372f451fc5ba4910b577ac6fe133337e93169be48b6d6'],
    ['icp-infinity.svg','64f4576c27dd0ff9815ddd63e9eeb589f1e80d3c81bb5d6baf3c3f537cc46934'],
    ['favicon.ico','56912bdc9d6c862359db56a018b7242dcefe634f28f0e1099c170bccb63ed36b'],
  ]) {
    const data = await readFile(publicFile(name));
    assert.equal(createHash('sha256').update(data).digest('hex'), hash, name);
    assert.ok(data.length < 20000, name);
    if (name.endsWith('.ico')) assert.deepEqual([...data.subarray(0,4)], [0,0,1,0]);
    else {
      assert.match(data.toString(), /<svg[^>]*xmlns="http:\/\/www.w3.org\/2000\/svg"/);
      assert.doesNotMatch(data.toString(), /<script\b|<foreignObject\b|\son\w+=|(?:href|src)="https?:|url\(["']?https?:/i);
    }
  }
});

test('explanations expose their requested external resources as separate actions', () => {
  for (const url of ['https://github.com/aodl/Event_Horizon','https://www.jupiter-faucet.com/','https://internetcomputer.org/']) {
    assert.ok(html.includes(`href="${url}"`)); assert.ok(!nav.includes(url));
  }
});
