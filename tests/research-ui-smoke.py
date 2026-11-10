"""Exercise the production HTML/JS in isolated Edge with a local fake Tauri bridge.

No GitHub or model requests are sent. Screenshots and DOM logs go to a temp folder.
Usage: python tests/research-ui-smoke.py [output-directory]
"""
import html as html_module
import json
import pathlib
import re
import subprocess
import sys
import tempfile

root = pathlib.Path(__file__).resolve().parents[1]
out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(tempfile.mkdtemp(prefix="gh-finder-ui-"))
out.mkdir(parents=True, exist_ok=True)
html = (root / "dist/index.html").read_text(encoding="utf-8")
html = re.sub(r'<script src="([^"]+)"></script>', lambda m: "<script>" + (root / "dist" / m[1]).read_text(encoding="utf-8") + "</script>", html)
html = re.sub(r'<link[^>]+href="(research.css)"[^>]*>', lambda m: "<style>" + (root / "dist" / m[1]).read_text(encoding="utf-8") + "</style>", html)
bridge = r'''<script>
const qaCalls=[],qaErrors=[];let qaListener=()=>{},qaResult=null,qaCancelMode=false,qaReject=null;
window.addEventListener('error',e=>qaErrors.push(e.message));window.addEventListener('unhandledrejection',e=>qaErrors.push(String(e.reason)));
const qaAssessment=id=>({id,verdict:'match',relevance:90,category:'app',reason:'本地界面测试：含所需功能',gaps:[],evidence:[]});
function qaSeed(session){return{session,source:'qa-mock',query:'批量改名',plan:{},queries:['批量改名','batch rename'],warnings:[],stats:{unique_count:45,shortlisted_count:20,assessed_count:20,model_calls:5,total_model_calls:5,prompt_tokens:500,completion_tokens:200,total_prompt_tokens:500,total_completion_tokens:200},items:Array.from({length:45},(_,n)=>({id:n+1,full_name:'UI-test/candidate-'+(n+1),description:'仅供本地界面测试，不是实际搜索结果。',language:'Python',stargazers_count:n,selected_for_ai:n<20,prefilter_score:1/(n+1),original_rank:n,assessment:n<20?qaAssessment(n+1):null,prefilter_reasons:['GitHub 第 '+(n+1)+' 位'],recalled_by:['批量改名']}))};}
function qaApply(ids){for(const id of ids){const c=qaResult.items.find(c=>c.id===id);c.selected_for_ai=true;c.assessment=qaAssessment(id);}const calls=Math.ceil(ids.length/5);qaResult.stats={...qaResult.stats,assessed_count:qaResult.items.filter(c=>c.assessment).length,shortlisted_count:qaResult.items.filter(c=>c.selected_for_ai).length,model_calls:calls,total_model_calls:qaResult.stats.total_model_calls+calls,prompt_tokens:50*calls,completion_tokens:30*calls,total_prompt_tokens:qaResult.stats.total_prompt_tokens+50*calls,total_completion_tokens:qaResult.stats.total_completion_tokens+30*calls};}
window.__TAURI__={core:{invoke:async(n,a)=>{qaCalls.push({n,a});switch(n){
case 'app_preferences':return{proxy_mode:'system',proxy_url:'http://127.0.0.1:7897',sample_dir:'C:\\QA\\Samples'};
case 'github_status':return{connected:false};case 'model_settings':return{configured:true,model:'deepseek-flash'};
case 'smart_search':qaResult=qaSeed(a.session);return structuredClone(qaResult);
case 'research_session':return structuredClone(qaResult);
case 'continue_research':{
 const ids=(a.candidateId==null?qaResult.items.filter(c=>!c.assessment).slice(0,a.reviewLimit):qaResult.items.filter(c=>c.id===a.candidateId&&!c.assessment)).map(c=>c.id);
 if(qaCancelMode){qaCancelMode=false;qaApply(ids.slice(0,5));qaListener({payload:{session:a.session,stage:'judging',message:'本地测试：已完成第一批',stats:structuredClone(qaResult.stats),items:structuredClone(qaResult.items.filter(c=>ids.slice(0,5).includes(c.id)))}});return await new Promise((resolve,reject)=>{qaReject=reject;});}
 await new Promise(r=>setTimeout(r,60));qaApply(ids);return structuredClone(qaResult);
}
case 'cancel_smart_search':if(qaReject){const reject=qaReject;qaReject=null;reject('搜索已取消');}return;
default:return{};
}}},event:{listen:async(name,callback)=>{qaListener=callback;return()=>{};}}};
</script>'''
html = html.replace('<head>', '<head>' + bridge)
actions = r'''<script>
window.addEventListener('load',()=>setTimeout(async()=>{
const mode='MODE',checks=[],$=s=>document.querySelector(s),all=s=>[...document.querySelectorAll(s)];
const check=(name,ok)=>{if(!ok)throw Error(name);checks.push(name);};
const tick=()=>new Promise(r=>setTimeout(r,100));
const reviews=()=>qaCalls.filter(c=>c.n==='continue_research');
try{
check('startup error free',!qaErrors.length);
$('#q').value='批量改名';
if(mode==='early-cancel'){
 const attempt=window.researchUI.search();await window.researchUI.cancel();await attempt;
 check('stop before startup sends no paid search',!qaCalls.some(c=>c.n==='smart_search'));
 check('controls recovered after early stop',!$('#researchStart').disabled);
 document.documentElement.dataset.qa=JSON.stringify({mode,ok:true,checks,errors:qaErrors});return;
}
await window.researchUI.search();
check('next batch and cost visible',!$('#reviewMoreBar').hidden&&$('#reviewMore').textContent.includes('20')&&$('#reviewMoreBudget').textContent.includes('4000'));
check('initial results preserved',all('#results .row').length===20);
$('#showAllCandidates').click();check('tail candidates accessible',all('#results .row').length===45);
if(mode==='continuation'){
 const tail=$('.review-project[data-id="45"]');check('single project action visible',!!tail);tail.click();await tick();
 check('tail project dispatched alone',reviews().length===1&&reviews()[0].a.candidateId===45);
 check('completed project action removed',!$('.review-project[data-id="45"]'));
 check('single project usage shown',$('#researchUsage').textContent.includes('最近一次：1 次')&&$('#researchUsage').textContent.includes('累计：6 次'));
 $('#reviewMore').click();$('#reviewMore').click();await tick();
 check('double click sends one request',reviews().length===2);
 check('next batch defaults to twenty',reviews()[1].a.reviewLimit===20&&reviews()[1].a.candidateId===null);
 check('prior and new judgments retained',qaResult.stats.assessed_count===41&&all('.review-project').length===4);
 check('remaining batch shrinks to four',$('#reviewMore').textContent.includes('4')&&$('#reviewMoreBudget').textContent.includes('1000'));
 await window.researchUI.reviewMore(45);check('completed project not recharged',reviews().length===2);
 $('#reviewMore').click();await tick();check('exhaustion disables action',$('#reviewMore').disabled&&all('.review-project').length===0);
 check('no second search',qaCalls.filter(c=>c.n==='smart_search').length===1);
}else if(mode==='cancel'){
 qaCancelMode=true;const run=window.researchUI.reviewMore();await tick();
 check('busy controls disabled',$('#reviewMore').disabled&&all('.review-project').every(b=>b.disabled));
 $('#researchCancel').click();await run;
 check('stopped batch restored',qaResult.stats.assessed_count===25&&all('.review-project').length===20);
 check('resume enabled after stop',!$('#reviewMore').disabled);
 await window.researchUI.reviewMore();check('resume finishes remaining only',qaResult.stats.assessed_count===45&&reviews().length===2);
}else if(mode==='stale'){
 $('#q').value='不同的需求';$('#q').dispatchEvent(new Event('input'));
 await window.researchUI.reviewMore();check('changed query cannot review old pool',reviews().length===0&&$('#reviewMoreBar').hidden);
}else if(mode==='preview'){
 // Leave the next batch and per-project actions visible for visual inspection.
 const row=$('.review-project').closest('.row');$('#results').style.scrollBehavior='auto';row.scrollIntoView({block:'center',behavior:'instant'});await tick();
}
check('runtime error free',!qaErrors.length);
document.documentElement.dataset.qa=JSON.stringify({mode,ok:true,checks,errors:qaErrors});
}catch(e){document.documentElement.dataset.qa=JSON.stringify({mode,ok:false,checks,errors:[String(e),...qaErrors]});}
},200));
</script>'''
edge = pathlib.Path(r'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe')
if not edge.is_file(): raise SystemExit('Microsoft Edge is required for the local UI smoke test')
failed = False
for mode in ['continuation', 'cancel', 'early-cancel', 'stale', 'preview']:
    page = out / (mode + '.html')
    page.write_text(html.replace('</body>', actions.replace('MODE', mode) + '</body>'), encoding='utf-8')
    p = subprocess.run([str(edge), '--headless=new', '--disable-gpu', '--no-first-run', '--disable-background-networking',
                        f'--user-data-dir={out / ("profile-" + mode)}', '--window-size=1040,800', '--virtual-time-budget=3500',
                        f'--screenshot={out / (mode + ".png")}', '--dump-dom', page.as_uri()], capture_output=True, timeout=40)
    dom = p.stdout.decode('utf-8', errors='replace')
    (out / (mode + '.dom.html')).write_text(dom, encoding='utf-8')
    match = re.search(r'data-qa="([^"]*)"', dom)
    if not match:
        failed = True
        print(mode + ': missing QA result')
        continue
    result = json.loads(html_module.unescape(match[1]))
    failed |= not result['ok']
    print(json.dumps(result, ensure_ascii=True), flush=True)
print('Screenshots: ' + str(out))
raise SystemExit(1 if failed else 0)
