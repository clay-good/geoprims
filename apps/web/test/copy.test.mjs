// The result panel's copy formats (web/tool-app). Each one is checked against
// a real result, because a value pasted somewhere else has to carry enough
// with it to say where it came from, and the agent call has to be a call an
// agent can actually make.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { FORMATS, copyText } from '../src/lib/copy.js';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const ID = 'aviation.altimetry.density-altitude';
const tool = catalog.tools.find((t) => t.id === ID);
const args = (tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0]).input;
const href = 'https://geoprims.com/aviation/altimetry/density-altitude/#v1:abc';

let result;
test('setup: run the worked example', async () => {
  result = JSON.parse(await host.invoke(ID, JSON.stringify(args)));
  assert.equal(result.ok, true);
});

test('the value copies as the reader sees it', () => {
  assert.equal(copyText('value', { answer: '7,932 ft', result, tool, args, href }), '7,932 ft');
});

test('the sentence carries its reference, so a pasted claim is traceable', () => {
  const text = copyText('sentence', { answer: '', result, tool, args, href });
  assert.ok(text.startsWith(result.summary), text);
  assert.ok(text.endsWith(`(geoprims ${tool.id} ${tool.version})`), text);
});

test('the agent call is a call an agent can make, and it reproduces the answer', async () => {
  const text = copyText('agent-call', { answer: '', result, tool, args, href });
  const call = JSON.parse(text);
  assert.equal(call.name, 'geoprims_run');
  assert.equal(call.arguments.id, ID);
  const again = JSON.parse(await host.invoke(call.arguments.id, JSON.stringify(call.arguments.args)));
  assert.deepEqual(again.result, result.result, 'the copied call gives a different answer');
  // It is the shape the page prints for developers, so both teach the same call.
  const printed = /<pre class="agent-call">([\s\S]*?)<\/pre>/.exec(readFileSync(join(dist, 'aviation/altimetry/density-altitude/index.html'), 'utf8'))[1];
  const unescaped = printed.replaceAll('&quot;', '"').replaceAll('&amp;', '&');
  assert.deepEqual(JSON.parse(unescaped), call, 'the copied call and the printed call differ');
});

test('the JSON copy is the whole envelope, warnings and provenance included', () => {
  const copied = JSON.parse(copyText('json', { answer: '', result, tool, args, href }));
  assert.deepEqual(copied, result);
  assert.ok(copied.meta.model, 'the copy carries the model');
  assert.ok(Array.isArray(copied.meta.warnings), 'the copy carries the warnings');
});

test('the link copies the permalink, not the bare page', () => {
  assert.equal(copyText('link', { answer: '', result, tool, args, href }), href);
});

test('an unknown format is a mistake, not a silent empty clipboard', () => {
  assert.throws(() => copyText('nope', { answer: '', result, tool, args, href }), /no copy format nope/);
});

test('every format the panel offers has a button on the page', () => {
  const html = readFileSync(join(dist, 'aviation/altimetry/density-altitude/index.html'), 'utf8');
  const actions = /<div class="actions">([\s\S]*?)<\/div>/.exec(html)[1];
  // Value, sentence, link, and the agent call are one click from the answer.
  for (const [format, label] of [['value', 'Copy'], ['sentence', 'Copy sentence'], ['link', 'Share link'], ['agent-call', 'Copy agent call']]) {
    assert.ok(actions.includes(label), `${format} has no button: ${actions.replace(/<[^>]+>/g, ' ')}`);
  }
  assert.ok(FORMATS.includes('json'), 'the JSON format is still offered');
});
