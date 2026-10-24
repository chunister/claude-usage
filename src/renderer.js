const $ = (id) => document.getElementById(id);
const copy = {
  en: { usageTab:'Usage',dashboard:'Dashboard',settingsTab:'Settings',session:'Current session',week:'This week',title:'Your Claude usage',loading:'Loading usage…',refresh:'Refresh',used:'% used',fiveWindow:'5-hour window',sevenWindow:'7-day window',noReset:'Reset time unavailable',reset:'Resets',within:'Your usage is on track.',watch:'Keep an eye on your usage limit.',cached:'Cached data',checking:'Refreshing…',connected:'Check that Claude Code is signed in.',updated:'Updated',auth:'Claude Code is not signed in. Sign in on this machine.',settings:'Settings',language:'Language',interval:'Refresh interval',minutes:'minutes',seconds:'seconds',help:'Minimum 5 minutes. The usage service may still apply its own limits.',tray:'Tray indicator',sessionUsage:'Session usage',weeklyUsage:'Weekly usage',bothUsage:'Session + week (5H + W)',launch:'Start when I sign in',save:'Save settings',saved:'Settings saved',minInterval:'Interval must be at least 5 minutes.',serverError:'Could not refresh usage.',historyTitle:'Usage history',sessionNow:'Session now',weekNow:'Week now',chartTitle:'Utilization history',samples:'samples',legendSession:'Session (5h)',legendWeek:'Week (7d)',historyNote:'History is collected on this device from now on. All collected history is kept.',noHistory:'Not enough history yet. Keep Claude Usage running to fill this chart.',periodLabel:'Period',daysGroup:'Days',monthsGroup:'Months',yearsGroup:'Years',last:'Last',dayUnit:'day',daysUnit:'days',monthUnit:'month',monthsUnit:'months',yearUnit:'year',yearsUnit:'years',allTime:'All time',exportHistory:'Export history',importHistory:'Import history',exported:'History file downloaded.',imported:'Imported history. Total samples: ',historyError:'Could not read that history file.' },
  pt: { usageTab:'Consumo',dashboard:'Painel',settingsTab:'Configurações',session:'Sessão atual',week:'Esta semana',title:'Seu uso do Claude',loading:'Carregando consumo…',refresh:'Atualizar',used:'% usado',fiveWindow:'Janela de 5 horas',sevenWindow:'Janela de 7 dias',noReset:'Horário de reinício indisponível',reset:'Reinicia',within:'Seu uso está dentro do previsto.',watch:'Acompanhe seu limite de uso.',cached:'Dados em cache',checking:'Atualizando…',connected:'Confira se o Claude Code está conectado.',updated:'Atualizado',auth:'Claude Code não conectado. Entre no Claude Code nesta máquina.',settings:'Configurações',language:'Idioma',interval:'Intervalo de atualização',minutes:'minutos',seconds:'segundos',help:'Mínimo de 5 minutos. O serviço de uso ainda pode aplicar limites próprios.',tray:'Indicador da bandeja',sessionUsage:'Consumo da sessão',weeklyUsage:'Consumo semanal',bothUsage:'Sessão + semana (5H + W)',launch:'Iniciar ao entrar no sistema',save:'Salvar configurações',saved:'Configurações salvas',minInterval:'O intervalo mínimo é de 5 minutos.',serverError:'Não foi possível atualizar o consumo.',historyTitle:'Histórico de uso',sessionNow:'Sessão agora',weekNow:'Semana agora',chartTitle:'Histórico de utilização',samples:'amostras',legendSession:'Sessão (5h)',legendWeek:'Semana (7d)',historyNote:'O histórico será coletado neste dispositivo a partir de agora e mantido sem prazo para expirar.',noHistory:'Ainda não há histórico suficiente. Mantenha o Claude Usage aberto para preencher o gráfico.',periodLabel:'Período',daysGroup:'Dias',monthsGroup:'Meses',yearsGroup:'Anos',last:'Últimos',dayUnit:'dia',daysUnit:'dias',monthUnit:'mês',monthsUnit:'meses',yearUnit:'ano',yearsUnit:'anos',allTime:'Todo o período',exportHistory:'Exportar histórico',importHistory:'Importar histórico',exported:'Arquivo do histórico baixado.',imported:'Histórico importado. Total de amostras: ',historyError:'Não foi possível ler esse arquivo de histórico.' },
  es: { usageTab:'Uso',dashboard:'Panel',settingsTab:'Configuración',session:'Sesión actual',week:'Esta semana',title:'Tu uso de Claude',loading:'Cargando uso…',refresh:'Actualizar',used:'% usado',fiveWindow:'Ventana de 5 horas',sevenWindow:'Ventana de 7 días',noReset:'Hora de reinicio no disponible',reset:'Se reinicia',within:'Tu uso va según lo previsto.',watch:'Vigila tu límite de uso.',cached:'Datos en caché',checking:'Actualizando…',connected:'Comprueba que Claude Code tenga la sesión iniciada.',updated:'Actualizado',auth:'Claude Code no tiene una sesión iniciada. Inicia sesión en este equipo.',settings:'Configuración',language:'Idioma',interval:'Intervalo de actualización',minutes:'minutos',seconds:'segundos',help:'Mínimo 5 minutos. El servicio de uso puede aplicar sus propios límites.',tray:'Indicador de la bandeja',sessionUsage:'Uso de la sesión',weeklyUsage:'Uso semanal',bothUsage:'Sesión + semana (5H + W)',launch:'Iniciar al iniciar sesión',save:'Guardar configuración',saved:'Configuración guardada',minInterval:'El intervalo mínimo es de 5 minutos.',serverError:'No se pudo actualizar el uso.',historyTitle:'Historial de uso',sessionNow:'Sesión ahora',weekNow:'Semana ahora',chartTitle:'Historial de utilización',samples:'muestras',legendSession:'Sesión (5h)',legendWeek:'Semana (7d)',historyNote:'El historial se recopila en este dispositivo desde ahora y se conserva indefinidamente.',noHistory:'Aún no hay suficiente historial. Mantén Claude Usage abierto para completar el gráfico.',periodLabel:'Periodo',daysGroup:'Días',monthsGroup:'Meses',yearsGroup:'Años',last:'Últimos',dayUnit:'día',daysUnit:'días',monthUnit:'mes',monthsUnit:'meses',yearUnit:'año',yearsUnit:'años',allTime:'Todo el tiempo',exportHistory:'Exportar historial',importHistory:'Importar historial',exported:'Archivo del historial descargado.',imported:'Historial importado. Total de muestras: ',historyError:'No se pudo leer ese archivo de historial.' }
};
let language = 'en', t = copy.en, currentSettings = { language:'en', refreshSeconds:300, launchAtLogin:false, trayDisplay:'session' };
let usageHistory = [], chartRange = '7d', latestUsage = null;

function applyLanguage(code) {
  language = ['en','pt','es'].includes(code) ? code : 'en'; t = copy[language];
  document.documentElement.lang = language === 'pt' ? 'pt-BR' : language;
  $('usage-tab').textContent=t.usageTab; $('settings-tab').textContent=t.settingsTab;
  $('dashboard-tab').textContent=t.dashboard; $('dashboard-heading').textContent=t.historyTitle;
  $('dash-session-label').textContent=t.sessionNow; $('dash-week-label').textContent=t.weekNow;
  $('chart-title').textContent=t.chartTitle; $('legend-session').textContent=t.legendSession; $('legend-week').textContent=t.legendWeek;
  $('history-note').textContent=t.historyNote;
  $('export-history').textContent=t.exportHistory; $('import-history').textContent=t.importHistory;
  $('period-label').textContent=t.periodLabel; populatePeriods();
  $('session-label').textContent=t.session; $('week-label').textContent=t.week;
  $('headline').textContent=t.title; $('refresh').title=t.refresh; $('refresh').setAttribute('aria-label',t.refresh);
  $('settings-heading').textContent=t.settings; $('language-label').textContent=t.language;
  $('interval-label').textContent=t.interval; $('interval-help').textContent=t.help;
  $('tray-label').textContent=t.tray; $('launch-label').textContent=t.launch;
  $('save-settings').textContent=t.save;
  $('interval-unit').options[0].textContent=t.minutes; $('interval-unit').options[1].textContent=t.seconds;
  $('tray-display').options[0].textContent=t.sessionUsage; $('tray-display').options[1].textContent=t.weeklyUsage; $('tray-display').options[2].textContent=t.bothUsage;
  $('five-reset').textContent=t.fiveWindow; $('week-reset').textContent=t.sevenWindow;
}
function populatePeriods() {
  const select=$('history-period'),selected=chartRange;
  select.replaceChildren();
  const addGroup=(label,unit,count,singular,plural)=>{
    const group=document.createElement('optgroup');group.label=label;
    for(let i=1;i<=count;i++){const option=document.createElement('option');option.value=`${i}${unit}`;option.textContent=`${t.last} ${i} ${i===1?singular:plural}`;group.append(option);}
    select.append(group);
  };
  addGroup(t.daysGroup,'d',7,t.dayUnit,t.daysUnit);
  addGroup(t.monthsGroup,'m',12,t.monthUnit,t.monthsUnit);
  addGroup(t.yearsGroup,'y',5,t.yearUnit,t.yearsUnit);
  const all=document.createElement('option');all.value='all';all.textContent=t.allTime;select.append(all);
  select.value=selected;
}
function percent(value) { const number=Number(value); return Number.isFinite(number)?Math.max(0,Math.min(100,Math.round(number<=1?number*100:number))):null; }
function resetText(value) {
  if(!value)return t.noReset;
  const date=new Date(value); if(Number.isNaN(date.getTime()))return t.noReset;
  return `${t.reset} ${new Intl.DateTimeFormat(document.documentElement.lang,{weekday:'long',hour:'2-digit',minute:'2-digit'}).format(date)}`;
}
function render(data) {
  const {usage,lastError,refreshing,settings,history=[]}=data;
  usageHistory=history; latestUsage=usage; renderDashboard(usage);
  if(settings){ currentSettings=settings; syncForm(settings); if(settings.language!==language)applyLanguage(settings.language); }
  if(!usage){ $('headline').textContent=lastError?.startsWith('Claude Code não conectado.')?t.auth:(lastError||t.loading); $('status').textContent=refreshing?t.checking:t.connected; return; }
  const five=percent(usage.five_hour?.utilization),week=percent(usage.seven_day?.utilization);
  $('five-value').textContent=five==null?'—':`${five}${t.used}`; $('week-value').textContent=week==null?'—':`${week}${t.used}`;
  $('five-bar').style.width=`${five??0}%`; $('week-bar').style.width=`${week??0}%`;
  $('five-reset').textContent=resetText(usage.five_hour?.resets_at); $('week-reset').textContent=resetText(usage.seven_day?.resets_at);
  $('headline').textContent=((five??0)<70&&(week??0)<70)?t.within:t.watch;
  $('status').textContent=lastError?`${t.cached} · ${lastError}`:(refreshing?t.checking:`${t.updated} ${new Date().toLocaleTimeString(document.documentElement.lang,{hour:'2-digit',minute:'2-digit'})}`);
}
function renderDashboard(usage) {
  const five=percent(usage?.five_hour?.utilization),week=percent(usage?.seven_day?.utilization);
  $('dash-session').textContent=five==null?'—':`${five}%`;
  $('dash-week').textContent=week==null?'—':`${week}%`;
  const now=Date.now(),start=periodStart(chartRange,now);
  let points=usageHistory.filter(sample=>sample.timestamp<=now&&(start==null||sample.timestamp>=start));
  $('history-count').textContent=`${points.length} ${t.samples}`;
  const host=$('usage-chart');
  if(points.length<2){host.innerHTML=`<div class="chart-empty">${t.noHistory}</div>`;return;}
  const limit=80;
  if(points.length>limit) points=Array.from({length:limit},(_,i)=>points[Math.round(i*(points.length-1)/(limit-1))]);
  const width=520,height=220,left=38,right=10,top=10,bottom=26,plotW=width-left-right,plotH=height-top-bottom;
  const firstTimestamp=points[0].timestamp,timeSpan=Math.max(1,points[points.length-1].timestamp-firstTimestamp);
  const pathFor=(key)=>{
    const chunks=[];let chunk=[];
    points.forEach(sample=>{
      const value=sample[key];
      if(value==null){if(chunk.length)chunks.push(chunk);chunk=[];return;}
      const x=left+((sample.timestamp-firstTimestamp)/timeSpan)*plotW,y=top+(1-Math.max(0,Math.min(100,value))/100)*plotH;
      chunk.push(`${chunk.length?'L':'M'}${x.toFixed(1)},${y.toFixed(1)}`);
    });if(chunk.length)chunks.push(chunk);
    return chunks.map(part=>part.join(' ')).join(' ');
  };
  const grid=[0,50,100].map(value=>{const y=top+(1-value/100)*plotH;return `<line class="chart-grid" x1="${left}" y1="${y}" x2="${width-right}" y2="${y}"/><text class="chart-axis" x="${left-8}" y="${y+4}" text-anchor="end">${value}%</text>`;}).join('');
  const first=new Date(points[0].timestamp),last=new Date(points[points.length-1].timestamp);
  const dateOptions=chartRange.endsWith('d')?{hour:'2-digit'}:chartRange.endsWith('y')||chartRange==='all'?{month:'short',year:'2-digit'}:{month:'short',day:'numeric'};
  const dateLabel=(date)=>new Intl.DateTimeFormat(document.documentElement.lang,dateOptions).format(date);
  host.innerHTML=`<svg viewBox="0 0 ${width} ${height}" role="img" aria-label="${t.chartTitle}">${grid}<path class="chart-line session-line" d="${pathFor('fiveHour')}"/><path class="chart-line week-line" d="${pathFor('sevenDay')}"/><text class="chart-axis" x="${left}" y="${height-5}">${dateLabel(first)}</text><text class="chart-axis" x="${width-right}" y="${height-5}" text-anchor="end">${dateLabel(last)}</text></svg>`;
}
function periodStart(period,now) {
  if(period==='all')return null;
  const match=period.match(/^(\d+)([dmy])$/);if(!match)return null;
  const amount=Number(match[1]),unit=match[2],date=new Date(now);
  if(unit==='d')return now-amount*24*60*60*1000;
  const day=date.getDate();date.setDate(1);
  if(unit==='m')date.setMonth(date.getMonth()-amount);
  else date.setFullYear(date.getFullYear()-amount);
  const lastDay=new Date(date.getFullYear(),date.getMonth()+1,0).getDate();
  date.setDate(Math.min(day,lastDay));return date.getTime();
}
function syncForm(settings) {
  $('language').value=settings.language; $('tray-display').value=settings.trayDisplay;
  $('launch-login').checked=settings.launchAtLogin;
  const seconds=Number(settings.refreshSeconds)||300;
  if(seconds%60===0&&seconds>=60){$('interval-unit').value='min';$('interval-value').value=seconds/60;}
  else{$('interval-unit').value='sec';$('interval-value').value=seconds;}
  setIntervalBounds();
}
function setIntervalBounds() {
  const minutes=$('interval-unit').value==='min';
  $('interval-value').min=minutes?'5':'300';
  $('interval-value').max=minutes?'60':'3600';
}
function showPage(page) {
  const isSettings=page==='settings',isDashboard=page==='dashboard'; $('usage-page').hidden=isSettings||isDashboard; $('dashboard-page').hidden=!isDashboard; $('settings-page').hidden=!isSettings;
  $('usage-tab').classList.toggle('active',!isSettings&&!isDashboard); $('dashboard-tab').classList.toggle('active',isDashboard); $('settings-tab').classList.toggle('active',isSettings);
}
$('usage-tab').addEventListener('click',()=>showPage('usage'));
$('dashboard-tab').addEventListener('click',()=>showPage('dashboard'));
$('settings-tab').addEventListener('click',()=>showPage('settings'));
$('history-period').addEventListener('change',event=>{chartRange=event.target.value;renderDashboard(latestUsage);});
$('language').addEventListener('change',event=>applyLanguage(event.target.value));
$('interval-unit').addEventListener('change',event=>{
  const input=$('interval-value'),oldUnit=event.target.value==='min'?'sec':'min';
  const oldSeconds=Number(input.value)*(oldUnit==='min'?60:1);
  input.value=event.target.value==='min'?Math.max(1,Math.round(oldSeconds/60)):oldSeconds;
  setIntervalBounds();
});
$('settings-form').addEventListener('submit',async event=>{
  event.preventDefault();
  const amount=Number($('interval-value').value),seconds=amount*($('interval-unit').value==='min'?60:1);
  if(!Number.isFinite(seconds)||seconds<300){$('save-status').textContent=t.minInterval;return;}
  const next={language:$('language').value,refreshSeconds:Math.min(3600,seconds),launchAtLogin:$('launch-login').checked,trayDisplay:$('tray-display').value};
  currentSettings=await window.claudeUsage.saveSettings(next); applyLanguage(currentSettings.language); syncForm(currentSettings);
  $('save-status').textContent=t.saved;
});
window.claudeUsage.onUpdate(render);
$('refresh').addEventListener('click',()=>window.claudeUsage.refresh());
$('export-history').addEventListener('click',async()=>{
  const samples=await window.claudeUsage.exportHistory();
  const blob=new Blob([JSON.stringify(samples,null,2)],{type:'application/json'});
  const url=URL.createObjectURL(blob),link=document.createElement('a');
  link.href=url;link.download='claude-usage-history.json';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
  $('history-status').textContent=t.exported;
});
$('import-history').addEventListener('click',()=>$('history-file').click());
$('history-file').addEventListener('change',async event=>{
  const file=event.target.files?.[0];if(!file)return;
  const status=$('history-status');status.textContent=t.checking;
  try{
    const parsed=JSON.parse(await file.text()),samples=Array.isArray(parsed)?parsed:parsed.history;
    if(!Array.isArray(samples))throw new Error('Invalid history format');
    await window.claudeUsage.beginHistoryImport();
    for(let offset=0;offset<samples.length;offset+=5000){
      await window.claudeUsage.appendHistoryImport(samples.slice(offset,offset+5000));
      status.textContent=`${t.importHistory} ${Math.min(offset+5000,samples.length)} / ${samples.length}`;
    }
    await window.claudeUsage.finishHistoryImport();
    usageHistory=await window.claudeUsage.exportHistory();renderDashboard(latestUsage);
    status.textContent=t.imported+usageHistory.length;
  }catch(error){status.textContent=`${t.historyError} ${error?.message||''}`;}
  event.target.value='';
});
window.claudeUsage.getSettings().then(settings=>{currentSettings=settings;applyLanguage(settings.language);syncForm(settings);});
