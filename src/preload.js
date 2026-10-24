const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

window.claudeUsage = {
  onUpdate: (callback) => listen('usage:update', (event) => callback(event.payload)),
  refresh: () => invoke('refresh_usage'),
  getSettings: () => invoke('get_settings'),
  saveSettings: (settings) => invoke('save_settings', { settings }),
  exportHistory: () => invoke('export_usage_history'),
  beginHistoryImport: () => invoke('begin_history_import'),
  appendHistoryImport: (samples) => invoke('append_history_import', { samples }),
  finishHistoryImport: () => invoke('finish_history_import')
};
