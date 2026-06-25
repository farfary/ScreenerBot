// Splash + boot-error screen logic for the Electron loading window.
//
// Loaded as an external script so it complies with the page CSP
// (`script-src 'self'`). The splash is shown while the backend starts; if the
// backend reports a fatal startup error (SCREENERBOT_ERROR), the main process
// pushes a structured payload here and we render the dedicated error screen.

(function () {
  // Apply the last-run theme (passed by the main process as ?theme=light|dark)
  // so the splash/loading screen renders in the same theme as the last session
  // instead of always dark. Runs before the electronAPI guard since it only
  // needs the query string. The window stays hidden until did-finish-load, so
  // there is no flash even though boot.js loads at the end of <body>.
  try {
    const t = new URLSearchParams(window.location.search).get('theme');
    if (t === 'light' || t === 'dark') {
      document.documentElement.setAttribute('data-theme', t);
    }
  } catch (e) { /* query unavailable — keep default */ }

  if (!window.electronAPI) return;

  // Version badge.
  window.electronAPI
    .getVersion()
    .then((version) => {
      document.getElementById('version').textContent = 'v' + version;
    })
    .catch(() => {});

  // Loading status updates.
  window.electronAPI.onLoadingStatus((status) => {
    const el = document.getElementById('status');
    if (el) el.textContent = status;
  });

  // Fatal startup error → show the boot-error screen.
  window.electronAPI.onBootError((payload) => {
    renderBootError(payload || {});
  });

  const SUBTITLES = {
    wallet_mismatch: 'A different wallet was detected',
    port_in_use: 'A required network port is busy',
    lock_held: 'ScreenerBot is already running',
    config_invalid: 'Configuration problem',
    directory_setup: 'Storage problem',
    generic: 'Startup error'
  };

  function renderBootError(payload) {
    document.querySelector('.splash-screen').classList.add('hidden');

    document.getElementById('bootErrorTitle').textContent =
      payload.title || 'ScreenerBot could not start';
    document.getElementById('bootErrorSubtitle').textContent =
      SUBTITLES[payload.code] || SUBTITLES.generic;
    document.getElementById('bootErrorDetail').textContent =
      payload.detail || 'The backend stopped unexpectedly.';

    const remedyWrap = document.getElementById('bootErrorRemedyWrap');
    if (payload.remedy) {
      document.getElementById('bootErrorRemedy').textContent = payload.remedy;
      remedyWrap.hidden = false;
    } else {
      remedyWrap.hidden = true;
    }

    document.getElementById('bootErrorLogPath').textContent = payload.log_path
      ? 'Log file: ' + payload.log_path
      : '';

    const actions = document.getElementById('bootErrorActions');
    actions.innerHTML = '';

    if (payload.recovery && payload.recovery.action === 'reset_wallet_data') {
      actions.appendChild(
        makeButton('Reset wallet data & restart', 'primary', async (btn) => {
          btn.disabled = true;
          btn.textContent = 'Working...';
          try {
            await window.electronAPI.bootResetWalletData();
          } catch (e) {
            btn.disabled = false;
            btn.textContent = 'Reset wallet data & restart';
          }
        })
      );
    }

    actions.appendChild(
      makeButton('Open logs folder', 'secondary', () => {
        window.electronAPI.bootOpenLogs();
      })
    );

    actions.appendChild(
      makeButton('Copy details', 'secondary', (btn) => {
        const text = [
          payload.title || '',
          '',
          payload.detail || '',
          '',
          payload.remedy ? 'How to fix:\n' + payload.remedy : '',
          payload.log_path ? '\nLog file: ' + payload.log_path : ''
        ].join('\n');
        navigator.clipboard
          .writeText(text)
          .then(() => {
            btn.textContent = 'Copied';
            setTimeout(() => {
              btn.textContent = 'Copy details';
            }, 1500);
          })
          .catch(() => {});
      })
    );

    actions.appendChild(
      makeButton('Quit', 'ghost', () => {
        window.electronAPI.bootQuit();
      })
    );

    document.getElementById('bootError').classList.add('visible');
  }

  function makeButton(label, variant, onClick) {
    const btn = document.createElement('button');
    btn.className = 'boot-btn ' + variant;
    btn.textContent = label;
    btn.addEventListener('click', () => onClick(btn));
    return btn;
  }
})();
