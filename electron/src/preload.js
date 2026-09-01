const { contextBridge, ipcRenderer } = require('electron');

/**
 * Expose a safe API to the renderer process
 * This allows the web content to interact with the Electron app
 */
contextBridge.exposeInMainWorld('electronAPI', {
  // Window controls
  minimize: () => ipcRenderer.invoke('app:minimize'),
  maximize: () => ipcRenderer.invoke('app:maximize'),
  close: () => ipcRenderer.invoke('app:close'),
  isMaximized: () => ipcRenderer.invoke('app:is-maximized'),
  onMaximizeChange: (callback) => {
    const handler = (event, isMax) => callback(isMax);
    ipcRenderer.on('window:maximize-change', handler);
    return () => ipcRenderer.removeListener('window:maximize-change', handler);
  },
  
  // Zoom controls (returns new zoom level)
  zoomIn: () => ipcRenderer.invoke('app:zoom-in'),
  zoomOut: () => ipcRenderer.invoke('app:zoom-out'),
  zoomReset: () => ipcRenderer.invoke('app:zoom-reset'),
  getZoomLevel: () => ipcRenderer.invoke('app:get-zoom-level'),
  
  // Fullscreen controls
  toggleFullscreen: () => ipcRenderer.invoke('app:toggle-fullscreen'),
  isFullscreen: () => ipcRenderer.invoke('app:is-fullscreen'),
  onFullscreenChange: (callback) => {
    const handler = (event, isFull) => callback(isFull);
    ipcRenderer.on('window:fullscreen-change', handler);
    return () => ipcRenderer.removeListener('window:fullscreen-change', handler);
  },
  
  // App info
  getVersion: () => ipcRenderer.invoke('app:get-version'),
  getShellInfo: () => ipcRenderer.invoke('app:get-shell-info'),
  quitForUpdate: () => ipcRenderer.invoke('app:quit-for-update'),
  onCheckForUpdates: (callback) => {
    const handler = () => callback();
    ipcRenderer.on('updates:check', handler);
    return () => ipcRenderer.removeListener('updates:check', handler);
  },

  // Persist the UI theme so the next launch's splash + window match it.
  saveTheme: (theme) => ipcRenderer.invoke('theme:set', theme),

  // Loading status listener (returns cleanup function)
  onLoadingStatus: (callback) => {
    const handler = (event, status) => callback(status);
    ipcRenderer.on('loading:status', handler);
    return () => ipcRenderer.removeListener('loading:status', handler);
  },

  // Splash update panel: progress of a staged update being applied at launch
  onUpdateProgress: (callback) => {
    const handler = (event, payload) => callback(payload);
    ipcRenderer.on('update:progress', handler);
    return () => ipcRenderer.removeListener('update:progress', handler);
  },

  // Boot-error screen: receive a structured fatal startup error payload
  onBootError: (callback) => {
    const handler = (event, payload) => callback(payload);
    ipcRenderer.on('boot:error', handler);
    return () => ipcRenderer.removeListener('boot:error', handler);
  },

  // Boot-error recovery/actions
  bootResetWalletData: () => ipcRenderer.invoke('boot:reset-wallet-data'),
  bootOpenLogs: () => ipcRenderer.invoke('boot:open-logs'),
  bootQuit: () => ipcRenderer.invoke('boot:quit'),
  
  // Platform info
  platform: process.platform,

  // Check if running in Electron
  isElectron: true
});

/**
 * Promo Studio channel — exposed only when the app was launched by the owner-only
 * capture driver. A normal session never has `window.promoAPI`, so the dashboard's
 * capture runtime cannot be driven from a real install.
 */
if (process.env.SCREENERBOT_PROMO_CONTROL === '1') {
  contextBridge.exposeInMainWorld('promoAPI', {
    onCommand: (callback) => {
      const handler = (event, payload) => callback(payload);
      ipcRenderer.on('promo:command', handler);
      return () => ipcRenderer.removeListener('promo:command', handler);
    },
    sendResult: (payload) => ipcRenderer.send('promo:result', payload)
  });
}
