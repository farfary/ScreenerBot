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

  // Shell text for the current language. The same call supplies the document
  // language and direction, applied before the page is first shown.
  const shellText = window.electronAPI.getShellStrings() || {};
  const strings = shellText.strings || {};
  if (shellText.locale) document.documentElement.lang = shellText.locale;
  if (shellText.dir) document.documentElement.dir = shellText.dir;

  function text(id) {
    return typeof strings[id] === 'string' ? strings[id] : id;
  }

  document.querySelectorAll('[data-l10n-id]').forEach((el) => {
    el.textContent = text(el.getAttribute('data-l10n-id'));
  });

  // Version badge.
  window.electronAPI
    .getVersion()
    .then((version) => {
      document.getElementById('version').textContent = 'v' + version;
    })
    .catch(() => {});

  // Launch state: `{ message, detail }`. The headline says what the launch is
  // doing; the detail is only present when there is something worth adding, and
  // its row is reserved in the layout either way so text never moves the mark.
  window.electronAPI.onLoadingStatus((status) => {
    const message = status && status.message ? status.message : '';
    const detail = status && status.detail ? status.detail : '';

    const statusEl = document.getElementById('status');
    if (statusEl && message) statusEl.textContent = message;

    const detailEl = document.getElementById('statusDetail');
    if (detailEl) detailEl.textContent = detail;
  });

  // Fatal startup error → show the boot-error screen.
  window.electronAPI.onBootError((payload) => {
    renderBootError(payload || {});
  });

  const SUBTITLE_IDS = {
    wallet_mismatch: 'desktop-boot-subtitle-wallet-mismatch',
    port_in_use: 'desktop-boot-subtitle-port-in-use',
    lock_held: 'desktop-boot-subtitle-lock-held',
    config_invalid: 'desktop-boot-subtitle-config-invalid',
    directory_setup: 'desktop-boot-subtitle-directory-setup',
    generic: 'desktop-boot-subtitle-generic'
  };

  function renderBootError(payload) {
    document.getElementById('splashScreen').classList.add('hidden');

    // The payload is finished text in its own locale; keep the page's language
    // and direction consistent with it.
    if (payload.locale) document.documentElement.lang = payload.locale;
    if (payload.dir) document.documentElement.dir = payload.dir;

    document.getElementById('bootErrorTitle').textContent =
      payload.title || text('desktop-boot-title-fallback');
    document.getElementById('bootErrorSubtitle').textContent =
      text(SUBTITLE_IDS[payload.code] || SUBTITLE_IDS.generic);
    document.getElementById('bootErrorDetail').textContent =
      payload.detail || text('desktop-boot-detail-fallback');

    const remedyWrap = document.getElementById('bootErrorRemedyWrap');
    if (payload.remedy) {
      document.getElementById('bootErrorRemedy').textContent = payload.remedy;
      remedyWrap.hidden = false;
    } else {
      remedyWrap.hidden = true;
    }

    // The path is a machine value: keep it in its own left-to-right run.
    const logPathEl = document.getElementById('bootErrorLogPath');
    logPathEl.textContent = '';
    if (payload.log_path) {
      const pathEl = document.createElement('bdi');
      pathEl.dir = 'ltr';
      pathEl.textContent = payload.log_path;
      logPathEl.append(text('desktop-boot-log-file-label') + ' ', pathEl);
    }

    const actions = document.getElementById('bootErrorActions');
    actions.innerHTML = '';

    if (payload.recovery && payload.recovery.action === 'reset_wallet_data') {
      actions.appendChild(
        makeButton(text('desktop-boot-action-reset-wallet'), 'primary', async (btn) => {
          btn.disabled = true;
          btn.textContent = text('desktop-boot-action-working');
          try {
            await window.electronAPI.bootResetWalletData();
          } catch (e) {
            btn.disabled = false;
            btn.textContent = text('desktop-boot-action-reset-wallet');
          }
        })
      );
    }

    actions.appendChild(
      makeButton(text('desktop-boot-action-open-logs'), 'secondary', () => {
        window.electronAPI.bootOpenLogs();
      })
    );

    actions.appendChild(
      makeButton(text('desktop-boot-action-copy'), 'secondary', (btn) => {
        const text = [
          payload.title || '',
          '',
          payload.detail || '',
          '',
          payload.remedy ? text('desktop-boot-remedy-label') + '\n' + payload.remedy : '',
          payload.log_path ? '\n' + text('desktop-boot-log-file-label') + ' ' + payload.log_path : ''
        ].join('\n');
        navigator.clipboard
          .writeText(text)
          .then(() => {
            btn.textContent = text('desktop-boot-action-copied');
            setTimeout(() => {
              btn.textContent = text('desktop-boot-action-copy');
            }, 1500);
          })
          .catch(() => {});
      })
    );

    actions.appendChild(
      makeButton(text('desktop-boot-action-quit'), 'ghost', () => {
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
