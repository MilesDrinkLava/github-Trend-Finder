/* Search and explicit, budgeted follow-up reviews on the current GitHub pool. */
(() => {
  const $ = s => document.querySelector(s), native = !!window.__TAURI__;
  const invoke = (n, a) => window.__TAURI__.core.invoke(n, a);
  const esc = t => String(t ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  let session = '', searchQuery = '', busy = false, operation = 0, stopRequested = false;
  let candidates = new Map(), stats = {}, warnings = [], plan = null, ready = Promise.resolve();
  const labels = {match:'较符合', partial:'部分符合', uncertain:'待确认', irrelevant:'不太符合'};
  const enabled = () => !$('#githubOnly').checked;
  const status = t => { $('#researchStatus').textContent = t; };
  const sameQuery = () => !!session && searchQuery === $('#q').value.trim() && enabled();
  const pending = () => [...candidates.values()].filter(c => !c.assessment).length;

  function counts() {
    const left = pending(), next = Math.min(left, Number($('#reviewLimit').value));
    $('#researchCounts').textContent = `GitHub 找到 ${stats.unique_count || candidates.size} 个；已送 AI ${stats.shortlisted_count || 0} 个；已完成 ${stats.assessed_count || 0} 个。`;
    $('#researchUsage').textContent = `最近一次：${stats.model_calls || 0} 次调用，输入 ${stats.prompt_tokens || 0}、输出 ${stats.completion_tokens || 0} tokens。当前搜索累计：${stats.total_model_calls ?? stats.model_calls ?? 0} 次，输入 ${stats.total_prompt_tokens ?? stats.prompt_tokens ?? 0}、输出 ${stats.total_completion_tokens ?? stats.completion_tokens ?? 0} tokens。`;
    const recommended = window.researchView.select([...candidates.values()], 'recommended', 'all', 'relevance', 100).length;
    $('#showRecommendations').textContent = `推荐项目${candidates.size ? ' ' + recommended : ''}`;
    $('#showAllCandidates').textContent = `全部 GitHub 结果${candidates.size ? ' ' + candidates.size : ''}`;
    $('#reviewMoreBar').hidden = !candidates.size || !sameQuery();
    $('#reviewRemaining').textContent = left ? `还有 ${left} 个项目未完成分析` : '这一批 GitHub 候选已全部分析';
    $('#reviewMoreBudget').textContent = left ? `下一批最多 ${Math.ceil(next / 5)} 次调用、${Math.ceil(next / 5) * 1000} 输出 token；单个项目最多 1 次、1000 输出 token。` : '可以重新搜索，获取新的 GitHub 候选。';
    $('#reviewMore').textContent = next ? `继续分析 ${next} 个` : '全部分析完成';
    $('#reviewMore').disabled = busy || !next || !sameQuery();
    $('#researchStart').disabled = busy;
    $('#researchRefresh').disabled = busy;
    $('#researchCancel').hidden = !busy;
    document.querySelectorAll('.review-project').forEach(b => { b.disabled = busy || !sameQuery(); });
  }
  function budgetHint() {
    const n = Number($('#reviewLimit').value);
    $('#reviewBudgetHint').textContent = `首次最多分析 ${n} 个，最多 ${1 + Math.ceil(n / 5)} 次调用、${600 + Math.ceil(n / 5) * 1000} 输出 token。后续每次追加也最多 ${n} 个，由你点击启动。`;
    counts();
  }
  function warn(w) {
    warnings = w || [];
    $('#researchWarnings').hidden = !warnings.length;
    $('#researchWarnings').textContent = warnings.slice(0, 2).join('；');
  }
  function render() {
    const list = $('#results'), scroll = list.scrollTop, selected = $('#results .sel')?.dataset.url;
    const open = new Set([...document.querySelectorAll('#results details[open]')].map(x => x.closest('.row').dataset.url));
    const rows = window.researchView.select([...candidates.values()], $('#researchFilter').value, $('#researchCategory').value, $('#researchSort').value, 100);
    $('#resultTabs').hidden = !candidates.size;
    $('#showRecommendations').classList.toggle('on', $('#researchFilter').value === 'recommended');
    $('#showAllCandidates').classList.toggle('on', $('#researchFilter').value === 'all');
    list.innerHTML = rows.map((c, i) => {
      const a = c.assessment, url = 'https://github.com/' + c.full_name, badge = a ? labels[a.verdict] : c.selected_for_ai ? 'GitHub 初选' : '未送 AI';
      return `<article class="row research-card${selected ? selected === url ? ' sel' : '' : i === 0 ? ' sel' : ''}" data-url="${esc(url)}">
        <div class="research-card-head"><strong>${esc(c.full_name)}</strong><span class="verdict ${esc(a?.verdict || 'pending')}">${esc(badge)}</span></div>
        <p class="research-description">${esc(c.description || '仓库未提供描述')}</p>${a ? `<p class="research-reason">${esc(a.reason)}</p>` : ''}
        <div class="research-meta"><span>★ ${Number(c.stargazers_count || c.stars || 0).toLocaleString()}</span><span>${esc(c.language || '')}</span>${c.archived ? '<span>已归档</span>' : ''}
        ${!a ? `<button class="quiet-btn review-project" data-id="${c.id}" title="仅分析这个项目：最多 1 次模型调用、1000 输出 token" aria-label="分析 ${esc(c.full_name)}，最多 1 次调用和 1000 输出 token">分析此项目</button>` : ''}
        <button class="quiet-btn open-repo">在 GitHub 打开</button></div>
        <details${open.has(url) ? ' open' : ''}><summary>查看判断依据</summary><div class="research-evidence">${a?.gaps?.length ? `<p>缺口或未验证：${esc(a.gaps.join('；'))}</p>` : ''}
        ${(a?.evidence || []).map(e => `<blockquote>${esc(e.quote)}<cite>${esc(e.source === 'readme' ? '项目说明' : e.source === 'topics' ? '项目标签' : e.source === 'license' ? '许可证元数据' : '项目描述')}</cite></blockquote>`).join('')}
        <p>GitHub 初筛：${esc((c.prefilter_reasons || []).join('；'))}</p><p>检索词：${esc((c.recalled_by || []).join(' / '))}</p>${!a ? '<p>这是 GitHub 候选，不代表已通过 AI 或运行验证。</p>' : ''}${c.readme_url ? '<p>已读取项目说明；未安装或执行项目代码。</p>' : ''}</div></details></article>`;
    }).join('') || '<p class="research-empty">暂时没有符合这组条件的推荐。可以查看“全部 GitHub 结果”，或继续分析下一批。</p>';
    list.scrollTop = scroll;
    document.querySelectorAll('#results .row').forEach(row => {
      row.addEventListener('click', () => { document.querySelectorAll('#results .sel').forEach(x => x.classList.remove('sel')); row.classList.add('sel'); });
      const openRepo = () => native ? invoke('open_url', {url:row.dataset.url}) : window.open(row.dataset.url, '_blank', 'noopener');
      row.querySelector('.open-repo').addEventListener('click', e => { e.stopPropagation(); openRepo(); });
      row.querySelector('.review-project')?.addEventListener('click', e => { e.stopPropagation(); reviewMore(Number(e.currentTarget.dataset.id)); });
      row.addEventListener('dblclick', e => { if (!e.target.closest('details,button')) openRepo(); });
    });
    counts();
  }
  function accept(d) {
    stats = d.stats || stats; plan = d.plan || plan; warn(d.warnings);
    for (const c of d.items || []) candidates.set(c.id, c);
    if (d.queries) $('#researchPlan').textContent = `检索词：${d.queries.join(' / ')}`;
    render();
  }
  function apply(p) {
    if (p.session !== session || !sameQuery() || !busy) return;
    stats = p.stats || stats;
    for (const c of p.items || []) candidates.set(c.id, c);
    status(p.message || '');
    if (p.items?.length) render(); else counts();
  }
  function invalidate() {
    if (busy && native) invoke('cancel_smart_search', {session}).catch(() => {});
    operation++; session = ''; searchQuery = ''; busy = false;
    if (!$('#q').value.trim()) { candidates.clear(); stats = {}; warn([]); $('#results').innerHTML = ''; $('#empty').hidden = false; $('#resultTabs').hidden = true; }
    counts();
    status($('#q').value.trim() ? '输入完成后点击搜索。' : '先从 GitHub 查找，再让 AI 精读少量候选。');
  }
  async function restore(current, run) {
    try { const d = await invoke('research_session', {session:current}); if (current === session && run === operation) accept(d); } catch (_) {}
  }
  async function cancel() {
    if (!busy || !native) return;
    stopRequested = true;
    status('正在停止；已完成的分析会保留。');
    try { await invoke('cancel_smart_search', {session}); } catch (e) { status(String(e)); }
  }
  async function search(force = false) {
    if (busy) return;
    const query = $('#q').value.trim();
    if (!query) { status('先描述你要找的项目。'); return; }
    $('#searchOptions').open = false;
    if (!enabled()) { invalidate(); $('#empty').hidden = true; $('#resultTabs').hidden = true; status('只搜索 GitHub，本次不会调用 AI。'); await liveSearch(query); return; }
    if (!native) { status('浏览器预览只展示界面；请在桌面版连接模型后搜索。'); return; }
    const run = ++operation;
    stopRequested = false;
    session = `search-${Date.now()}-${Math.random().toString(16).slice(2)}`; searchQuery = query;
    const current = session;
    candidates.clear(); stats = {}; plan = null; warn([]); $('#results').innerHTML = ''; $('#resultTabs').hidden = true;
    busy = true; counts(); $('#researchSettingsLink').hidden = true; $('#empty').hidden = true; $('#notice').hidden = true;
    status('正在准备中英文检索词…');
    try {
      await ready;
      if (current !== session || run !== operation) return;
      if (stopRequested) { status('已停止，本次未发送搜索请求。'); return; }
      const d = await invoke('smart_search', {query, target:Number($('#sampleBudget').value), reviewLimit:Number($('#reviewLimit').value), session:current, refresh:force === true});
      if (current !== session || run !== operation) return;
      accept(d);
      status(d.source === 'cache' ? '已使用分析缓存，本次没有调用 AI；可以继续分析剩余候选。' : `找到 ${candidates.size} 个项目，已分析 ${stats.assessed_count || 0} 个，结果已保存。`);
    } catch (e) {
      if (current !== session || run !== operation) return;
      await restore(current, run);
      if (current !== session || run !== operation) return;
      status(String(e)); $('#researchSettingsLink').hidden = !String(e).includes('Key') && !String(e).includes('密钥');
    } finally { if (current === session && run === operation) { busy = false; counts(); } }
  }
  async function reviewMore(candidateId = null) {
    if (busy || !native || !sameQuery()) return;
    if (candidateId !== null && (!candidates.has(candidateId) || candidates.get(candidateId).assessment)) return;
    if (!pending()) return;
    const current = session, run = ++operation, before = stats.assessed_count || 0;
    stopRequested = false;
    busy = true; counts(); $('#searchOptions').open = false;
    status(candidateId === null ? '准备分析下一批候选…' : '准备分析指定项目…');
    try {
      const d = await invoke('continue_research', {session:current, candidateId, reviewLimit:Number($('#reviewLimit').value)});
      if (current !== session || run !== operation) return;
      accept(d);
      const stopped = (d.warnings || []).find(w => w.startsWith('AI精读已停止：'));
      status(`本次新增完成 ${Math.max(0, stats.assessed_count - before)} 个，累计已分析 ${stats.assessed_count} / ${candidates.size} 个；${stopped || '结果已保存。'}`);
    } catch (e) {
      if (current !== session || run !== operation) return;
      await restore(current, run);
      if (current !== session || run !== operation) return;
      status(String(e));
    } finally { if (current === session && run === operation) { busy = false; counts(); } }
  }
  window.researchUI = {enabled, search, invalidate, reviewMore, cancel, settingsChanged:cancel};
  $('#researchStart').addEventListener('click', () => search());
  $('#researchRefresh').addEventListener('click', () => search(true));
  $('#researchCancel').addEventListener('click', cancel);
  $('#reviewMore').addEventListener('click', () => reviewMore());
  $('#githubOnly').addEventListener('change', () => { clearTimeout(debounceT); searchRequest++; gen++; if (fetchCtl) fetchCtl.abort(); invalidate(); });
  $('#researchSettingsLink').addEventListener('click', () => document.querySelector('.nav[data-view="settings"]').click());
  $('#showRecommendations').addEventListener('click', () => { $('#researchFilter').value = 'recommended'; render(); });
  $('#showAllCandidates').addEventListener('click', () => { $('#researchFilter').value = 'all'; render(); });
  for (const id of ['researchFilter', 'researchCategory', 'researchSort']) $('#' + id).addEventListener('input', render);
  $('#reviewLimit').addEventListener('change', budgetHint);
  $('#q').addEventListener('keydown', e => { if (e.key === 'Enter' && !e.isComposing && e.keyCode !== 229) { e.preventDefault(); e.stopImmediatePropagation(); search(); } });
  $('#researchExport').addEventListener('click', async () => {
    if (!native || !candidates.size || !sameQuery()) return;
    try { status(`结果已另存：${await invoke('export_research', {payload:JSON.stringify({query:searchQuery, plan, stats, warnings, items:[...candidates.values()]}, null, 2)})}`); } catch (e) { status(String(e)); }
  });
  if (native) { ready = window.__TAURI__.event.listen('research-progress', e => apply(e.payload)); ready.catch(e => status(`进度监听失败：${e}`)); }
  budgetHint();
})();
