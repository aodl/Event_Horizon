/* Independent copyable example. No licence grant: see README.md. */
(() => {
  if (window.initICSubnetMap) return;
  let active;
  function principal(value) {
    if (!/^(?:[a-z2-7]{5}-)*[a-z2-7]{1,5}$/.test(value) || value.length > 63) throw Error('Invalid subnet principal');
    const raw = value.replaceAll('-', ''), bytes = [];
    let buffer = 0, bits = 0;
    for (const char of raw) {
      buffer = (buffer << 5) | 'abcdefghijklmnopqrstuvwxyz234567'.indexOf(char);
      bits += 5;
      if (bits >= 8) { bits -= 8; bytes.push((buffer >>> bits) & 255); buffer &= (1 << bits) - 1; }
    }
    if (buffer || bytes.length < 5 || bytes.length > 33 || raw.length !== Math.ceil(bytes.length * 8 / 5)) throw Error('Invalid subnet principal');
    let crc = 0xffffffff;
    for (const byte of bytes.slice(4)) {
      crc ^= byte;
      for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
    }
    const checksum = ((bytes[0] << 24) | (bytes[1] << 16) | (bytes[2] << 8) | bytes[3]) >>> 0;
    if (((crc ^ 0xffffffff) >>> 0) !== checksum) throw Error('Invalid subnet principal checksum');
    return value;
  }
  window.initICSubnetMap = (host = document.querySelector('[data-ic-subnet-map]')) => {
    if (!host) return null;
    if (active) {
      if (active.host !== host) throw Error('This example supports one subnet widget per page');
      return active.api;
    }
    const subnet = encodeURIComponent(principal(host.dataset.icSubnetMap));
    if (!window.HTMLDialogElement?.prototype.showModal) return null;
    const dialog = document.createElement('dialog');
    dialog.className = 'ic-subnet-map';
    dialog.dataset.mode = 'minimised';
    dialog.setAttribute('aria-label', 'Fully on-chain');
    dialog.innerHTML = `<header><h2>Fully on-chain</h2>
      <button type="button" data-map-mode="minimised" aria-label="Minimise subnet map" title="Minimise subnet map"><span class="ic-map-icon ic-map-icon--minimise" aria-hidden="true"></span></button>
      <button type="button" data-map-mode="thumbnail" aria-label="Restore subnet thumbnail" title="Restore subnet thumbnail"><span class="ic-map-icon ic-map-icon--restore" aria-hidden="true"></span></button>
      <button type="button" data-map-mode="expanded" aria-label="Maximise subnet map" title="Maximise subnet map"><span class="ic-map-icon" aria-hidden="true"></span></button></header>
      <div class="ic-map-preview"><div class="ic-map-embed" inert aria-hidden="true">
        <iframe title="ICP Dashboard subnet map" tabindex="-1" sandbox="allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox" referrerpolicy="no-referrer"></iframe>
        <div class="ic-map-loading" role="status">Loading subnet map…</div></div></div>
      <div class="ic-map-attribution"><p>Subnet map provided by ICP Dashboard. <a target="_blank" rel="noopener noreferrer">View it on ICP Dashboard</a>.</p><p class="ic-map-status" role="status"></p></div>`;
    const frame = dialog.querySelector('iframe'), embed = dialog.querySelector('.ic-map-embed');
    const layer = dialog.querySelector('.ic-map-loading'), status = dialog.querySelector('.ic-map-status');
    dialog.querySelector('a').href = `https://dashboard.internetcomputer.org/network/subnet/${subnet}`;
    let previousMode = 'thumbnail', started = false, slowTimer, settleTimer;
    const clearTimers = () => { clearTimeout(slowTimer); clearTimeout(settleTimer); };
    const loaded = () => {
      if (!frame.getAttribute('src')) return;
      clearTimers();
      settleTimer = setTimeout(() => { layer.hidden = true; status.textContent = ''; }, 500);
    };
    const start = () => {
      if (started) return;
      started = true;
      frame.addEventListener('load', loaded);
      slowTimer = setTimeout(() => { layer.hidden = false; status.textContent = 'Taking longer than expected. Use the ICP Dashboard link above.'; }, 12000);
      frame.src = `https://dashboard.internetcomputer.org/embed/subnet-map?subnet=${subnet}`;
    };
    const layout = () => {
      const width = embed.getBoundingClientRect().width;
      const scale = dialog.dataset.mode === 'expanded' && width > 760 ? width / 760 : 1;
      if (scale > 1) { embed.style.setProperty('--ic-map-scale', scale); embed.style.setProperty('--ic-map-height', `${520 * scale}px`); embed.setAttribute('data-scaled', ''); }
      else { embed.removeAttribute('data-scaled'); embed.style.removeProperty('--ic-map-scale'); embed.style.removeProperty('--ic-map-height'); }
    };
    const present = mode => {
      const current = dialog.dataset.mode;
      if (current === mode) return;
      const expanded = mode === 'expanded', modalChange = expanded || current === 'expanded';
      if (expanded) previousMode = current;
      if (modalChange) dialog.close();
      dialog.dataset.mode = mode;
      embed.inert = !expanded;
      embed.setAttribute('aria-hidden', String(!expanded));
      frame.tabIndex = expanded ? 0 : -1;
      if (modalChange) expanded ? dialog.showModal() : dialog.show();
      layout();
      dialog.scrollTop = 0;
      if (mode !== 'minimised') start();
      dialog.querySelector(`[data-map-mode="${expanded ? 'minimised' : mode === 'minimised' ? 'thumbnail' : 'expanded'}"]`).focus({ preventScroll: true });
    };
    const click = event => {
      const control = event.target.closest('button[data-map-mode]');
      if (control && dialog.contains(control)) present(control.dataset.mapMode);
    };
    const cancel = event => { event.preventDefault(); present(previousMode); };
    dialog.addEventListener('click', click);
    dialog.addEventListener('cancel', cancel);
    host.append(dialog);
    dialog.open = true; // Non-modal startup without autofocus.
    const observer = window.ResizeObserver ? new ResizeObserver(layout) : null;
    observer?.observe(embed);
    const api = { dispose() {
      if (active?.api !== api) return;
      clearTimers(); observer?.disconnect();
      frame.removeEventListener('load', loaded);
      dialog.removeEventListener('click', click);
      dialog.removeEventListener('cancel', cancel);
      dialog.close(); dialog.remove(); active = undefined;
    } };
    active = { host, api };
    return api;
  };
  window.initICSubnetMap();
})();
