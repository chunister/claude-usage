const $ = (id) => document.getElementById(id);
const copy = {
  en: { usageTab:'Usage',settingsTab:'Settings',session:'Current session',week:'This week',title:'Your Claude usage',loading:'Loading usage…',refresh:'Refresh',used:'% used',fiveWindow:'5-hour window',sevenWindow:'7-day window',noReset:'Reset time unavailable',reset:'Resets',within:'Your usage is on track.',watch:'Keep an eye on your usage limit.',cached:'Cached data',checking:'Refreshing…',connected:'Check that Claude Code is signed in.',updated:'Updated',auth:'Claude Code is not signed in. Sign in on this machine.',settings:'Settings',language:'Language',interval:'Refresh interval',minutes:'minutes',seconds:'seconds',help:'Minimum 5 minutes. The usage service may still apply its own limits.',tray:'Tray indicator',sessionUsage:'Session usage',weeklyUsage:'Weekly usage',bothUsage:'Session and weekly',launch:'Start when I sign in',save:'Save settings',saved:'Settings saved',minInterval:'Interval must be at least 5 minutes.',serverError:'Could not refresh usage.' },
  pt: { usageTab:'Consumo',settingsTab:'Configurações',session:'Sessão atual',week:'Esta semana',title:'Seu uso do Claude',loading:'Carregando consumo…',refresh:'Atualizar',used:'% usado',fiveWindow:'Janela de 5 horas',sevenWindow:'Janela de 7 dias',noReset:'Horário de reinício indisponível',reset:'Reinicia',within:'Seu uso está dentro do previsto.',watch:'Acompanhe seu limite de uso.',cached:'Dados em cache',checking:'Atualizando…',connected:'Confira se o Claude Code está conectado.',updated:'Atualizado',auth:'Claude Code não conectado. Entre no Claude Code nesta máquina.',settings:'Configurações',language:'Idioma',interval:'Intervalo de atualização',minutes:'minutos',seconds:'segundos',help:'Mínimo de 5 minutos. O serviço de uso ainda pode aplicar limites próprios.',tray:'Indicador da bandeja',sessionUsage:'Consumo da sessão',weeklyUsage:'Consumo semanal',bothUsage:'Sessão e semana',launch:'Iniciar ao entrar no sistema',save:'Salvar configurações',saved:'Configurações salvas',minInterval:'O intervalo mínimo é de 5 minutos.',serverError:'Não foi possível atualizar o consumo.' },
  es: { usageTab:'Uso',settingsTab:'Configuración',session:'Sesión actual',week:'Esta semana',title:'Tu uso de Claude',loading:'Cargando uso…',refresh:'Actualizar',used:'% usado',fiveWindow:'Ventana de 5 horas',sevenWindow:'Ventana de 7 días',noReset:'Hora de reinicio no disponible',reset:'Se reinicia',within:'Tu uso va según lo previsto.',watch:'Vigila tu límite de uso.',cached:'Datos en caché',checking:'Actualizando…',connected:'Comprueba que Claude Code tenga la sesión iniciada.',updated:'Actualizado',auth:'Claude Code no tiene una sesión iniciada. Inicia sesión en este equipo.',settings:'Configuración',language:'Idioma',interval:'Intervalo de actualización',minutes:'minutos',seconds:'segundos',help:'Mínimo 5 minutos. El servicio de uso puede aplicar sus propios límites.',tray:'Indicador de la bandeja',sessionUsage:'Uso de la sesión',weeklyUsage:'Uso semanal',bothUsage:'Sesión y semana',launch:'Iniciar al iniciar sesión',save:'Guardar configuración',saved:'Configuración guardada',minInterval:'El intervalo mínimo es de 5 minutos.',serverError:'No se pudo actualizar el uso.' }
};
let language = 'en', t = copy.en, currentSettings = { language:'en', refreshSeconds:300, launchAtLogin:false, trayDisplay:'session' };

function applyLanguage(code) {
  language = ['en','pt','es'].includes(code) ? code : 'en'; t = copy[language];
  document.documentElement.lang = language === 'pt' ? 'pt-BR' : language;
  $('usage-tab').textContent=t.usageTab; $('settings-tab').textContent=t.settingsTab;
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
function percent(value) { const number=Number(value); return Number.isFinite(number)?Math.max(0,Math.min(100,Math.round(number<=1?number*100:number))):null; }
function resetText(value) {
  if(!value)return t.noReset;
  const date=new Date(value); if(Number.isNaN(date.getTime()))return t.noReset;
  return `${t.reset} ${new Intl.DateTimeFormat(document.documentElement.lang,{weekday:'long',hour:'2-digit',minute:'2-digit'}).format(date)}`;
}
function render(data) {
  const {usage,lastError,refreshing,settings}=data;
  if(settings){ currentSettings=settings; syncForm(settings); if(settings.language!==language)applyLanguage(settings.language); }
  if(!usage){ $('headline').textContent=lastError?.startsWith('Claude Code não conectado.')?t.auth:(lastError||t.loading); $('status').textContent=refreshing?t.checking:t.connected; return; }
  const five=percent(usage.five_hour?.utilization),week=percent(usage.seven_day?.utilization);
  $('five-value').textContent=five==null?'—':`${five}${t.used}`; $('week-value').textContent=week==null?'—':`${week}${t.used}`;
  $('five-bar').style.width=`${five??0}%`; $('week-bar').style.width=`${week??0}%`;
  $('five-reset').textContent=resetText(usage.five_hour?.resets_at); $('week-reset').textContent=resetText(usage.seven_day?.resets_at);
  $('headline').textContent=((five??0)<70&&(week??0)<70)?t.within:t.watch;
  $('status').textContent=lastError?`${t.cached} · ${lastError}`:(refreshing?t.checking:`${t.updated} ${new Date().toLocaleTimeString(document.documentElement.lang,{hour:'2-digit',minute:'2-digit'})}`);
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
  const isSettings=page==='settings'; $('usage-page').hidden=isSettings; $('settings-page').hidden=!isSettings;
  $('usage-tab').classList.toggle('active',!isSettings); $('settings-tab').classList.toggle('active',isSettings);
}
$('usage-tab').addEventListener('click',()=>showPage('usage'));
$('settings-tab').addEventListener('click',()=>showPage('settings'));
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
window.claudeUsage.getSettings().then(settings=>{currentSettings=settings;applyLanguage(settings.language);syncForm(settings);});
