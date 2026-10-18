import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile} from 'node:fs/promises';

const publicFile = name => new URL(`../public/${name}`, import.meta.url);
const html = await readFile(publicFile('index.html'), 'utf8');
const router = await readFile(new URL('../src/lib.rs', import.meta.url), 'utf8');
const canonicalOrigin = 'https://www.event-horizon-icp.com/';
const imageUrl = `${canonicalOrigin}og/event-horizon-preview-v1.jpg`;

function attributeValues(element, attribute) {
  return [...html.matchAll(new RegExp(`<${element}\\b[^>]*\\b${attribute}="([^"]+)"[^>]*>`, 'g'))]
    .map(match => match[1]);
}

function metaValues(attribute, key) {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return [...html.matchAll(new RegExp(`<meta\\s+${attribute}="${escaped}"\\s+content="([^"]*)">`, 'g'))]
    .map(match => match[1]);
}

function jpegDimensions(bytes) {
  assert.equal(bytes[0], 0xff);
  assert.equal(bytes[1], 0xd8);
  let offset = 2;
  const startOfFrame = new Set([0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf]);
  while (offset + 4 <= bytes.length) {
    if (bytes[offset++] !== 0xff) continue;
    while (bytes[offset] === 0xff) offset++;
    const marker = bytes[offset++];
    if (marker === 0xda || marker === 0xd9) break;
    if (marker === 0xd8 || (marker >= 0xd0 && marker <= 0xd7)) continue;
    const length = bytes.readUInt16BE(offset);
    assert.ok(length >= 2 && offset + length <= bytes.length, 'valid JPEG segment length');
    if (startOfFrame.has(marker)) {
      return {height: bytes.readUInt16BE(offset + 3), width: bytes.readUInt16BE(offset + 5)};
    }
    offset += length;
  }
  assert.fail('JPEG has no start-of-frame marker');
}

test('custom-domain ownership file is exact and explicitly certified as plain text', async () => {
  const ownership = await readFile(publicFile('.well-known/ic-domains'));
  assert.deepEqual(ownership, Buffer.from('www.event-horizon-icp.com\n'));
  assert.doesNotMatch(ownership.toString(), /^event-horizon-icp\.com$/m);
  assert.match(router, /path: "\.well-known\/ic-domains"\.to_string\(\),[\s\S]*?content_type: Some\("text\/plain; charset=utf-8"\.to_string\(\)\),[\s\S]*?headers: headers\(NO_CACHE, PAGE_CSP\)/);
});

test('approved preview JPEG bytes, dimensions and explicit certified media type are fixed', async () => {
  const preview = await readFile(publicFile('og/event-horizon-preview-v1.jpg'));
  assert.equal(preview.length, 173285);
  assert.equal(createHash('sha256').update(preview).digest('hex'), '1dd854f28501ce6706468e446fdb684d2b55d1d4119dc8b52cd1264c41dda3d2');
  assert.deepEqual(jpegDimensions(preview), {width: 1200, height: 630});
  assert.match(router, /path: "og\/event-horizon-preview-v1\.jpg"\.to_string\(\),[\s\S]*?content_type: Some\("image\/jpeg"\.to_string\(\)\),[\s\S]*?headers: headers\(REVALIDATE, PAGE_CSP\)/);
});

test('static canonical, Open Graph and X metadata use one permanent HTTPS origin', async () => {
  const canonicals = [...html.matchAll(/<link rel="canonical" href="([^"]+)">/g)].map(match => match[1]);
  assert.deepEqual(canonicals, [canonicalOrigin]);

  const requiredOg = new Map([
    ['og:type', 'website'],
    ['og:site_name', 'Event Horizon'],
    ['og:title', 'Event Horizon'],
    ['og:description', 'Ledger event signals for Internet Computer canisters, supported by productive ICP endowments.'],
    ['og:url', canonicalOrigin],
    ['og:image', imageUrl],
    ['og:image:secure_url', imageUrl],
    ['og:image:type', 'image/jpeg'],
    ['og:image:width', '1200'],
    ['og:image:height', '630'],
    ['og:image:alt', 'Event Horizon: a luminous black hole and accretion disk, with the EVH mark, a small planet silhouette and an ICP Ledger transaction snapshot.'],
  ]);
  for (const [key, value] of requiredOg) assert.deepEqual(metaValues('property', key), [value], key);

  const requiredTwitter = new Map([
    ['twitter:card', 'summary_large_image'],
    ['twitter:title', 'Event Horizon'],
    ['twitter:description', requiredOg.get('og:description')],
    ['twitter:image', imageUrl],
    ['twitter:image:alt', requiredOg.get('og:image:alt')],
  ]);
  for (const [key, value] of requiredTwitter) assert.deepEqual(metaValues('name', key), [value], key);

  assert.equal(new URL(requiredOg.get('og:url')).origin, new URL(requiredOg.get('og:image')).origin);
  assert.equal(requiredOg.get('og:image'), requiredTwitter.get('twitter:image'));
  assert.ok(requiredOg.get('og:description').length > 40);
  assert.ok(requiredOg.get('og:image:alt').length > 40);
  assert.doesNotMatch(html, /<(?:link|meta)\b[^>]+(?:icp\.net|icp0\.io)/i);
  assert.deepEqual(attributeValues('link', 'rel').filter(value => value === 'canonical'), ['canonical']);
  await readFile(publicFile(new URL(imageUrl).pathname.slice(1)));
});

test('metadata adds no remote runtime asset and keeps the pinned ICP API host independent', async () => {
  const [pricingClient, ...dependentClients] = await Promise.all([
    readFile(publicFile('pricing-client.js'), 'utf8'),
    readFile(publicFile('ledger-counter-client.js'), 'utf8'),
    readFile(publicFile('operating-status-client.js'), 'utf8'),
  ]);
  assert.match(pricingClient, /https:\/\/icp-api\.io/);
  for (const client of dependentClients) {
    assert.match(client, /import\s*\{\s*ICP_API_HOST\s*\}\s*from '\.\/pricing-client\.js'/);
    assert.doesNotMatch(client, /window\.location\.origin|event-horizon-icp\.com/);
  }
  assert.doesNotMatch(pricingClient, /window\.location\.origin|event-horizon-icp\.com/);
  assert.doesNotMatch(html, /<(?:script|img|object)\b[^>]+(?:src|data)="https:\/\//i);
  assert.doesNotMatch(html, /<link\b[^>]+rel="(?:stylesheet|preload)"[^>]+href="https:\/\//i);
});
