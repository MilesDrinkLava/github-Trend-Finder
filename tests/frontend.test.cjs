const {test} = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const path = require('node:path');
const html = fs.readFileSync(path.join(__dirname, '../dist/index.html'), 'utf8');

function harness() {
  const pending = [], rendered = [], notices = [];
  const classes = new Set();
  const elements = {
    '#q': {value: 'a'},
    '#results': {innerHTML: '', classList: {
      add: x => classes.add(x), remove: x => classes.delete(x)
    }},
    '#empty': {hidden: true},
  };
  const context = vm.createContext({
    AbortController,
    TAURI: true,
    window: {__TAURI__: {core: {invoke: () => new Promise((resolve, reject) => {
      pending.push({resolve, reject});
    })}}},
    $: key => elements[key],
    setNote: (...args) => notices.push(args),
    setConn: () => {},
    renderRows: rows => rendered.push(rows),
    rowFromItem: x => x,
  });
  vm.runInContext('let searchRequest = 0, gen = 0, fetchCtl = null;', context);
  const start = html.indexOf('async function liveSearch(');
  const end = html.indexOf('/* ── 热榜', start);
  vm.runInContext(html.slice(start, end), context);
  return {context, elements, pending, rendered, notices, classes};
}

test('obsolete search failure cannot overwrite newer results', async () => {
  const h = harness();
  const first = h.context.liveSearch('a');
  h.elements['#q'].value = 'b';
  const second = h.context.liveSearch('b');
  h.pending[1].resolve({items: [{full_name: 'new/result'}]});
  await second;
  h.pending[0].reject(new Error('old failure'));
  await first;
  assert.equal(h.rendered.length, 1);
  assert.equal(h.rendered[0][0].full_name, 'new/result');
  assert.equal(h.notices.some(x => x[1] === true), false);
});

test('old completion cannot clear newer loading state, even for identical text', async () => {
  const h = harness();
  const first = h.context.liveSearch('a');
  const second = h.context.liveSearch('a');
  h.pending[0].resolve({items: [{full_name: 'old/result'}]});
  await first;
  assert.equal(h.classes.has('loading'), true);
  assert.equal(h.rendered.length, 0);
  h.pending[1].resolve({items: [{full_name: 'new/result'}]});
  await second;
  assert.equal(h.classes.has('loading'), false);
  assert.equal(h.rendered[0][0].full_name, 'new/result');
});

test('clearing the search invalidates pending response', async () => {
  const h = harness();
  const first = h.context.liveSearch('a');
  h.elements['#q'].value = '';
  await h.context.liveSearch('');
  h.pending[0].resolve({items: [{full_name: 'old/result'}]});
  await first;
  assert.equal(h.rendered.length, 0);
  assert.equal(h.elements['#empty'].hidden, false);
  assert.equal(h.classes.has('loading'), false);
});

test('all inline scripts parse', () => {
  for (const match of html.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)) {
    assert.doesNotThrow(() => new vm.Script(match[1]));
  }
});

test('obsolete discovery failure cannot overwrite newer period', async () => {
  const pending = [], rendered = [];
  const list = {innerHTML: ''};
  const context = vm.createContext({
    TAURI: true, Date,
    window: {__TAURI__: {core: {invoke: () => new Promise((resolve, reject) => {
      pending.push({resolve, reject});
    })}}},
    $: () => list,
    hotTemplate: items => rendered.push(items),
    ageText: () => '',
    esc: x => x,
  });
  vm.runInContext('let hotRequest = 0, period = "day", lang = "all", translateOn = false; const hotCache = new Map(); const HOT_CACHE_TTL = 1800000;', context);
  const start = html.indexOf('async function renderHot(');
  const end = html.indexOf('/* ── 交互', start);
  vm.runInContext(html.slice(start, end), context);
  const first = context.renderHot();
  vm.runInContext('period = "week";', context);
  const second = context.renderHot();
  pending[1].resolve({items: [{full_name: 'new/week', description: '', stars: 1}]});
  await second;
  list.innerHTML = 'new list';
  pending[0].reject(new Error('old failure'));
  await first;
  assert.equal(rendered[0][0].n, 'new/week');
  assert.equal(list.innerHTML, 'new list');
});
