const {test}=require('node:test');
const assert=require('node:assert/strict');
const {select}=require('../dist/research-view.js');
const rows=[
  {id:1,full_name:'cn/中文工具',stars:0,original_rank:1,assessment:{verdict:'match',relevance:98,category:'app'}},
  {id:2,full_name:'popular/awesome',stars:500000,original_rank:0,assessment:{verdict:'irrelevant',relevance:2,category:'resource'}},
  {id:3,full_name:'cn/未分析',stars:0,original_rank:2,assessment:null},
];
test('default ranking preserves a zero-star Chinese match above popular noise',()=>{
  assert.equal(select(rows)[0].id,1);
  assert.equal(select(rows).length,3);
});
test('unassessed repositories are uncertain rather than irrelevant',()=>{
  assert.deepEqual(select(rows,'uncertain').map(x=>x.id),[3]);
  assert.deepEqual(select(rows,'irrelevant').map(x=>x.id),[2]);
});
test('filters and weights do not mutate or remove original samples',()=>{
  const before=JSON.stringify(rows);
  assert.equal(select(rows,'all','all','relevance',0)[0].id,2);
  assert.deepEqual(select(rows,'recommended','app').map(x=>x.id),[1]);
  assert.deepEqual(select(rows,'all','all','original').map(x=>x.id),[2,1,3]);
  assert.equal(JSON.stringify(rows),before);
});
test('new frontend scripts parse',()=>{
  const fs=require('node:fs'), vm=require('node:vm'), path=require('node:path');
  assert.doesNotThrow(()=>new vm.Script(fs.readFileSync(path.join(__dirname,'../dist/research.js'),'utf8')));
});
