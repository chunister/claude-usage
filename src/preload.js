const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

window.claudeUsage = {
  onUpdate: (callback) => listen('usage:update', (event) => callback(event.payload)),
  refresh: () => invoke('refresh_usage'),
  getSettings: () => invoke('get_settings'),
  saveSettings: (settings) => invoke('save_settings', { settings })
};
