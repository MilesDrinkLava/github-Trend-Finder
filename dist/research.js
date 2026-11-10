/* GitHub prefilter followed by a small, budgeted AI reading list. */
(() => {
  const $=s=>document.querySelector(s), native=!!window.__TAURI__;
  const invoke=(n,a)=>window.__TAURI__.core.invoke(n,a);
  const esc=t=>String(t??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  let session='',busy=false,candidates=new Map(),stats={},warnings=[],plan=null,ready=Promise.resolve();
  const labels={match:'较符合',partial:'部分符合',uncertain:'待确认',irrelevant:'不太符合'};
  const enabled=()=>!$('#githubOnly').checked;
  const status=t=>{$('#researchStatus').textContent=t;};
  function counts(){
    $('#researchCounts').textContent=`GitHub 找到 ${stats.unique_count||candidates.size} 个；初筛入选 ${stats.shortlisted_count||0} 个；AI 已精读 ${stats.assessed_count||0} 个。`;
    $('#researchUsage').textContent=`本次 AI：${stats.model_calls||0} 次；输入 ${stats.prompt_tokens||0}、输出 ${stats.completion_tokens||0} tokens。`;
    const recommended=window.researchView.select([...candidates.values()],'recommended','all','relevance',100).length;
    $('#showRecommendations').textContent=`推荐项目${candidates.size?' '+recommended:''}`;
    $('#showAllCandidates').textContent=`全部 GitHub 结果${candidates.size?' '+candidates.size:''}`;
  }
  function budgetHint(){const n=Number($('#reviewLimit').value);$('#reviewBudgetHint').textContent=`AI 最多精读 ${n} 个，最多 ${1+Math.ceil(n/5)} 次调用、${600+Math.ceil(n/5)*1000} 个输出 token；不会逐个分析全部检索结果。`;}
  function warn(w){warnings=w||[];$('#researchWarnings').hidden=!warnings.length;$('#researchWarnings').textContent=warnings.slice(0,2).join('；');}
  function render(){
    const list=$('#results'),scroll=list.scrollTop,selected=$('#results .sel')?.dataset.url;
    const open=new Set([...document.querySelectorAll('#results details[open]')].map(x=>x.closest('.row').dataset.url));
    const rows=window.researchView.select([...candidates.values()],$('#researchFilter').value,$('#researchCategory').value,$('#researchSort').value,100);
    $('#resultTabs').hidden=!candidates.size;
    $('#showRecommendations').classList.toggle('on',$('#researchFilter').value==='recommended');$('#showAllCandidates').classList.toggle('on',$('#researchFilter').value==='all');
    list.innerHTML=rows.map((c,i)=>{
      const a=c.assessment,url='https://github.com/'+c.full_name,badge=a?labels[a.verdict]:c.selected_for_ai?'GitHub 初选':'未送 AI';
      return `<article class="row research-card${selected?selected===url?' sel':'':i===0?' sel':''}" data-url="${esc(url)}">
        <div class="research-card-head"><strong>${esc(c.full_name)}</strong><span class="verdict ${esc(a?.verdict||'pending')}">${esc(badge)}</span></div>
        <p class="research-description">${esc(c.description||'仓库未提供描述')}</p>${a?`<p class="research-reason">${esc(a.reason)}</p>`:''}
        <div class="research-meta"><span>★ ${Number(c.stargazers_count||c.stars||0).toLocaleString()}</span><span>${esc(c.language||'')}</span>${c.archived?'<span>已归档</span>':''}<button class="quiet-btn open-repo">在 GitHub 打开</button></div>
        <details${open.has(url)?' open':''}><summary>查看判断依据</summary><div class="research-evidence">${a?.gaps?.length?`<p>缺口或未验证：${esc(a.gaps.join('；'))}</p>`:''}
        ${(a?.evidence||[]).map(e=>`<blockquote>${esc(e.quote)}<cite>${esc(e.source==='readme'?'项目说明':e.source==='topics'?'项目标签':'项目描述')}</cite></blockquote>`).join('')}
        <p>GitHub 初筛：${esc((c.prefilter_reasons||[]).join('；'))}</p><p>检索词：${esc((c.recalled_by||[]).join(' / '))}</p>${!a?'<p>这是 GitHub 候选，不代表已通过 AI 或运行验证。</p>':''}${c.readme_url?'<p>已读取项目说明；未安装或执行项目代码。</p>':''}</div></details></article>`;
    }).join('')||'<p class="research-empty">暂时没有符合这组条件的推荐。可以查看“全部 GitHub 结果”。</p>';
    list.scrollTop=scroll;
    document.querySelectorAll('#results .row').forEach(row=>{
      row.addEventListener('click',()=>{document.querySelectorAll('#results .sel').forEach(x=>x.classList.remove('sel'));row.classList.add('sel');});
      const openRepo=()=>native?invoke('open_url',{url:row.dataset.url}):window.open(row.dataset.url,'_blank','noopener');
      row.querySelector('.open-repo').addEventListener('click',e=>{e.stopPropagation();openRepo();});row.addEventListener('dblclick',e=>{if(!e.target.closest('details,button'))openRepo();});
    });counts();
  }
  function apply(p){if(p.session!==session||!enabled())return;stats=p.stats||stats;for(const c of p.items||[])candidates.set(c.id,c);status(p.message||'');if(p.items?.length)render();else counts();}
  function stop(){busy=false;$('#researchStart').disabled=false;$('#researchCancel').hidden=true;}
  function invalidate(){
    if(busy&&native)invoke('cancel_smart_search',{session}).catch(()=>{});session='';stop();
    if(!$('#q').value.trim()){candidates.clear();stats={};counts();$('#results').innerHTML='';$('#empty').hidden=false;$('#resultTabs').hidden=true;}
    status($('#q').value.trim()?'输入完成后点击搜索。':'先从 GitHub 查找，再让 AI 精读少量候选。');
  }
  async function search(force=false){
    const query=$('#q').value.trim();if(!query){status('先描述你要找的项目。');return;}
    $('#searchOptions').open=false;
    if(!enabled()){invalidate();$('#empty').hidden=true;$('#resultTabs').hidden=true;status('只搜索 GitHub，本次不会调用 AI。');await liveSearch(query);return;}
    if(!native){status('浏览器预览只展示界面；请在桌面版连接模型后搜索。');return;}
    await ready;if(busy)await invoke('cancel_smart_search',{session});
    session=`search-${Date.now()}-${Math.random().toString(16).slice(2)}`;const current=session;
    candidates.clear();stats={};plan=null;warn([]);counts();$('#results').innerHTML='';busy=true;
    $('#researchStart').disabled=true;$('#researchCancel').hidden=false;$('#researchSettingsLink').hidden=true;$('#empty').hidden=true;$('#notice').hidden=true;status('正在准备中英文检索词…');
    try{
      const d=await invoke('smart_search',{query,target:Number($('#sampleBudget').value),reviewLimit:Number($('#reviewLimit').value),session:current,refresh:force===true});if(current!==session)return;
      stats=d.stats;plan=d.plan;warn(d.warnings);for(const c of d.items||[])candidates.set(c.id,c);$('#researchPlan').textContent=`检索词：${(d.queries||[]).join(' / ')}`;
      status(d.source==='cache'?'已使用保存的分析缓存，本次没有调用 AI。':`从 ${candidates.size} 个项目中精读了 ${stats.assessed_count||0} 个，结果已保存。`);render();
    }catch(e){if(current!==session)return;status(String(e));$('#researchSettingsLink').hidden=false;}finally{if(current===session)stop();}
  }
  window.researchUI={enabled,search,invalidate};
  $('#researchStart').addEventListener('click',()=>search());$('#researchRefresh').addEventListener('click',()=>search(true));$('#researchCancel').addEventListener('click',()=>{invalidate();status('已停止。已取得的样本保留在样本文件夹。');});
  $('#githubOnly').addEventListener('change',()=>{clearTimeout(debounceT);searchRequest++;gen++;if(fetchCtl)fetchCtl.abort();invalidate();});
  $('#researchSettingsLink').addEventListener('click',()=>document.querySelector('.nav[data-view="settings"]').click());
  $('#showRecommendations').addEventListener('click',()=>{$('#researchFilter').value='recommended';render();});$('#showAllCandidates').addEventListener('click',()=>{$('#researchFilter').value='all';render();});
  for(const id of ['researchFilter','researchCategory','researchSort'])$('#'+id).addEventListener('input',render);$('#reviewLimit').addEventListener('change',budgetHint);
  $('#q').addEventListener('keydown',e=>{if(e.key==='Enter'&&!e.isComposing&&e.keyCode!==229){e.preventDefault();e.stopImmediatePropagation();search();}});
  $('#researchExport').addEventListener('click',async()=>{if(!native||!candidates.size)return;try{status(`结果已另存：${await invoke('export_research',{payload:JSON.stringify({query:$('#q').value,plan,stats,warnings,items:[...candidates.values()]},null,2)})}`);}catch(e){status(String(e));}});
  if(native){ready=window.__TAURI__.event.listen('research-progress',e=>apply(e.payload));ready.catch(e=>status(`进度监听失败：${e}`));}counts();budgetHint();
})();
