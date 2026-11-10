/* Pure presentation policy, shared by the UI and Node regression tests. */
(function(root){
  function select(candidates, filter='all', category='all', sort='relevance', weight=100){
    const w=Math.max(0,Math.min(100,Number.isFinite(weight)?weight:100))/100;
    return candidates.filter(c=>{
      const v=c.assessment?.verdict;
      if(filter==='recommended'&&(c.selected_for_ai!==undefined ? (!c.selected_for_ai||v==='irrelevant') : !['match','partial'].includes(v)))return false;
      if(filter==='uncertain'&&v!=='uncertain'&&c.assessment)return false;
      if(filter==='irrelevant'&&v!=='irrelevant')return false;
      return category==='all'||c.assessment?.category===category;
    }).sort((a,b)=>{
      if(sort==='original')return a.original_rank-b.original_rank;
      const stars=c=>Number(c.stargazers_count||c.stars||0);
      if(sort==='stars')return stars(b)-stars(a)||a.original_rank-b.original_rank;
      if(!a.assessment&&!b.assessment)return (b.prefilter_score||0)-(a.prefilter_score||0)||a.original_rank-b.original_rank;
      const score=c=>c.assessment ? w*c.assessment.relevance+(1-w)*Math.min(100,Math.log10(stars(c)+1)*20) : -1;
      return score(b)-score(a)||a.original_rank-b.original_rank;
    });
  }
  root.researchView={select};
  if(typeof module!=='undefined')module.exports=root.researchView;
})(typeof window!=='undefined'?window:globalThis);
