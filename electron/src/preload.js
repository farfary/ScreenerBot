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
  
  // Loading status listener (returns cleanup function)
  onLoadingStatus: (callback) => {
    const handler = (event, status) => callback(status);
    ipcRenderer.on('loading:status', handler);
    return () => ipcRenderer.removeListener('loading:status', handler);
  },
  
  // Platform info
  platform: process.platform,
  
  // Check if running in Electron
  isElectron: true
});
