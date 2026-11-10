/* V2 search session. Original Chinese is sent verbatim; sliders never issue requests. */
(() => {
  const $ = s => document.querySelector(s);
  const native = !!window.__TAURI__;
  const invoke = (name, args) => window.__TAURI__.core.invoke(name, args);
  const escape = text => String(text ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  let session = '', busy = false, candidates = new Map(), stats = {}, warnings = [], plan = null;
  let ready = Promise.resolve();
  const labels = {match:'相关',partial:'部分符合',uncertain:'待核实',irrelevant:'不相关'};
  const categories = {app:'应用',library:'库',framework:'框架',plugin:'插件',resource:'资源集合',example:'示例',other:'其他'};

  function enabled(){ return $('#searchMode').value === 'smart'; }
  function status(message){ $('#researchStatus').textContent = message; }
  function counts(){
    $('#researchCounts').textContent = `召回 ${stats.raw_count || 0} 条 · 去重 ${stats.unique_count || candidates.size} 个 · 已分析 ${stats.assessed_count || 0} 个 · 深读 ${stats.readme_count || 0} 个`;
    $('#researchUsage').textContent = `模型调用 ${stats.model_calls || 0} 次 · 输入 ${stats.prompt_tokens || 0} / 输出 ${stats.completion_tokens || 0} tokens`;
  }
  function render(){
    const scroll=$('#results').scrollTop;
    const selected=$('#results .sel')?.dataset.url;
    const open=new Set([...document.querySelectorAll('#results details[open]')].map(x=>x.closest('.row').dataset.url));
    const rows = window.researchView.select([...candidates.values()], $('#researchFilter').value,
      $('#researchCategory').value, $('#researchSort').value, Number($('#relevanceWeight').value));
    $('#weightValue').textContent = `${$('#relevanceWeight').value}% 相关度`;
    $('#results').innerHTML = rows.map((c,i) => {
      const a = c.assessment, verdict = a?.verdict;
      const name = c.full_name || '';
      const evidence = (a?.evidence || []).map(e => `<blockquote>${escape(e.quote)}<cite>${escape(e.source)}</cite></blockquote>`).join('');
      const url='https://github.com/'+name;
      return `<article class="row research-card${selected ? selected === url ? ' sel' : '' : i === 0 ? ' sel' : ''}" data-url="${escape(url)}">
        <div class="research-card-head"><strong>${escape(name)}</strong><span class="verdict ${escape(verdict || 'pending')}">${a ? labels[verdict] || '待核实' : '尚未分析'}</span></div>
        <p class="research-description">${escape(c.description || '仓库未提供描述')}</p>
        <div class="research-meta"><span>★ ${Number(c.stargazers_count || c.stars || 0).toLocaleString()}</span><span>${escape(c.language || '未知语言')}</span>${c.archived ? '<span>已归档</span>' : ''}<span>${escape(categories[a?.category] || '')}</span>${a ? `<span>模型相关度 ${a.relevance}/100</span>` : ''}</div>
        ${a ? `<p class="research-reason">${escape(a.reason)}</p>` : ''}
        ${a?.gaps?.length ? `<p class="research-gaps">缺口 / 未验证：${escape(a.gaps.join('；'))}</p>` : ''}
        <details${open.has(url)?' open':''}><summary>检索来源与判断依据</summary><div class="research-evidence"><p>召回词：${escape((c.recalled_by || []).join(' / '))}</p>${evidence || '<p>尚无通过原文校验的支持性引文</p>'}${c.readme_url ? `<p>已读取 README${c.readme_sha ? ` · ${escape(c.readme_sha.slice(0,8))}` : ''}</p>` : '<p>仅元数据判断，未读取源码或验证运行</p>'}</div></details>
      </article>`;
    }).join('') || '<p class="research-empty">当前筛选没有结果。可切换“全部样本”，查看尚未分析或被判为不相关的候选。</p>';
    $('#results').scrollTop=scroll;
    document.querySelectorAll('#results .row').forEach(row => {
      row.addEventListener('click', () => {document.querySelectorAll('#results .sel').forEach(x=>x.classList.remove('sel'));row.classList.add('sel');});
      row.addEventListener('dblclick', e => {if (!e.target.closest('details')) invoke('open_url',{url:row.dataset.url});});
    });
    counts();
  }
  function apply(progress){
    if (progress.session !== session || !enabled()) return;
    stats = progress.stats || stats;
    for (const c of progress.items || []) candidates.set(c.id, c);
    status(progress.message || '');
    render();
  }
  function stopState(){busy=false;$('#researchStart').disabled=false;$('#researchCancel').disabled=true;}
  function invalidate(){
    if (!enabled()) return;
    if (busy && native) invoke('cancel_smart_search',{session}).catch(()=>{});
    session='';stopState();
    if(!$('#q').value.trim()) {candidates.clear();stats={};counts();$('#results').innerHTML='';$('#empty').hidden=false;}
    status($('#q').value.trim() ? '需求已更新，点击“采样并筛选”开始新一轮' : '保留原文，扩展中英文查询，分批判断所有候选');
  }
  async function search(force=false){
    if (!enabled()) {liveSearch($('#q').value.trim());return;}
    const query=$('#q').value.trim();
    if (!query) {status('先描述你要找的项目');return;}
    if (!native) {status('智能采样需要桌面版；浏览器预览只显示界面');return;}
    await ready;
    if (busy) await invoke('cancel_smart_search',{session});
    session = `research-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const current=session;
    candidates.clear();stats={};warnings=[];plan=null;
    busy=true;$('#researchStart').disabled=true;$('#researchCancel').disabled=false;
    $('#empty').hidden=true;$('#notice').hidden=true;$('#researchWarnings').textContent='';
    status('正在生成中英文检索计划…');render();
    try {
      const data=await invoke('smart_search',{query,target:Number($('#sampleBudget').value),session:current,refresh:force===true});
      if (current !== session) return;
      stats=data.stats;warnings=data.warnings || [];plan=data.plan;
      for (const c of data.items || []) candidates.set(c.id,c);
      $('#researchWarnings').textContent=warnings.join('；');
      $('#researchPlan').textContent=`检索词：${(data.queries || []).join(' / ')}${plan?.requirements?.length ? '。需求条件：'+plan.requirements.join('；') : ''}`;
      status(`${data.source==='cache'?'使用 30 分钟内的分析缓存，本次未调用模型。':'检索与筛选完成。'}共 ${candidates.size} 个项目，未分析项目仍保留。`);
      render();
    } catch(e) {
      if (current !== session) return;
      status(String(e));
      $('#researchWarnings').textContent='本轮已取得的样本仍可查看；失败或未分析不等于不相关。';
    } finally {if (current===session)stopState();}
  }
  window.researchUI={enabled,search,invalidate};
  $('#researchStart').addEventListener('click', search);
  $('#researchRefresh').addEventListener('click',()=>search(true));
  $('#researchCancel').addEventListener('click',()=>{invalidate();status('已停止，已取得的样本和判断保留');});
  $('#searchMode').addEventListener('change',()=>{
    clearTimeout(debounceT);searchRequest++;gen++;if(fetchCtl)fetchCtl.abort();
    if (native) invoke('cancel_smart_search',{session}).catch(()=>{});
    session='';stopState();$('#researchPanel').hidden=!enabled();
    $('#empty').hidden=false;$('#results').innerHTML='';
    $('#researchStart').textContent=enabled()?'采样并筛选':'搜索原文';
    if(!enabled()&&$('#q').value.trim())liveSearch($('#q').value.trim());
  });
  for(const id of ['researchFilter','researchCategory','researchSort','relevanceWeight']) $('#'+id).addEventListener('input',render);
  $('#q').addEventListener('keydown',e=>{
    if(enabled()&&e.key==='Enter'&&!e.isComposing&&e.keyCode!==229){e.preventDefault();e.stopImmediatePropagation();search();}
  });
  async function refreshSettings(){
    if (!native) return;
    try {const s=await invoke('model_settings');$('#deepseekModel').value=s.model;
      $('#deepseekStatus').textContent=s.configured?'已配置密钥；智能搜索将发送检索需求和候选资料':'尚未配置密钥';}
    catch(e){$('#deepseekStatus').textContent=String(e);}
  }
  async function save(clear=false){
    if(!native)return;
    try {await invoke('save_model_settings',{apiKey:$('#deepseekKey').value,model:$('#deepseekModel').value,clearKey:clear});
      $('#deepseekKey').value='';await refreshSettings();}
    catch(e){$('#deepseekStatus').textContent=String(e);}
  }
  $('#deepseekSave').addEventListener('click',()=>save());
  $('#deepseekClear').addEventListener('click',()=>save(true));
  $('#deepseekTest').addEventListener('click',async()=>{
    if(!native)return;$('#deepseekStatus').textContent='正在检查连接…';
    try{$('#deepseekStatus').textContent=await invoke('test_model_connection');}
    catch(e){$('#deepseekStatus').textContent=String(e);}
  });
  $('#researchExport').addEventListener('click',async()=>{
    if(!native||!candidates.size)return;
    const payload=JSON.stringify({query:$('#q').value,plan,stats,warnings,items:[...candidates.values()]},null,2);
    try{const path=await invoke('export_research',{payload});status(`样本已保存：${path}`);}catch(e){status(String(e));}
  });
  if(native){ready=window.__TAURI__.event.listen('research-progress',e=>apply(e.payload));ready.catch(e=>status(`进度监听失败：${e}`));refreshSettings();}
  counts();
})();
