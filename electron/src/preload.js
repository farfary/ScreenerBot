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
  
  // Zoom controls
  zoomIn: () => ipcRenderer.invoke('app:zoom-in'),
  zoomOut: () => ipcRenderer.invoke('app:zoom-out'),
  zoomReset: () => ipcRenderer.invoke('app:zoom-reset'),
  
  // App info
  getVersion: () => ipcRenderer.invoke('app:get-version'),
  
  // Platform info
  platform: process.platform,
  
  // Check if running in Electron
  isElectron: true
});
