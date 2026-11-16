// Native details/name owns expansion. This helper only reveals fragment targets.
export function initSectionDisclosures(documentRef, windowRef) {
  const disclosures = [...documentRef.querySelectorAll('main > section > details.section-disclosure[name="event-horizon-sections"]')];
  let pending, settling;
  const cancelPositioning = () => {
    windowRef.cancelAnimationFrame(pending);
    windowRef.clearTimeout?.(settling);
  };
  const reveal = hash => {
    cancelPositioning();
    let target;
    try { target = documentRef.getElementById(decodeURIComponent(hash.slice(1))); } catch { return; }
    const details = disclosures.find(node => node.parentElement.contains(target));
    if (!details) return;
    const summary = details.querySelector('summary');
    const destination = target === details.parentElement ? summary : target;
    const startFocus = documentRef.activeElement;
    const position = (forceScroll = false) => {
      const active = documentRef.activeElement;
      if (active && active !== startFocus && active !== destination && active !== documentRef.body) return;
      const rect = destination.getBoundingClientRect();
      if (forceScroll || rect.top < 0 || rect.top >= windowRef.innerHeight) destination.scrollIntoView({ block: 'start', behavior: 'instant' });
      if (destination === summary || summary.contains(target)) summary.focus({ preventScroll: true });
      else if (target.matches('input, select, textarea, button, a[href], [tabindex]')) target.focus({ preventScroll: true });
    };
    details.open = true;
    pending = windowRef.requestAnimationFrame(() => {
      position();
      // details-content transitions are not exposed by getAnimations/events in
      // every engine. One bounded correction uses the actual CSS duration.
      const durations = windowRef.getComputedStyle?.(details, '::details-content').transitionDuration ?? '0s';
      const duration = Math.max(0, ...durations.split(',').map(value => parseFloat(value) * (value.trim().endsWith('ms') ? 1 : 1000)).filter(Number.isFinite));
      if (duration) pending = windowRef.requestAnimationFrame(() => {
        settling = windowRef.setTimeout(() => {
          pending = windowRef.requestAnimationFrame(() => {
            if (details.open) position(true);
          });
        }, duration);
      });
    });
  };
  const hashchange = () => reveal(windowRef.location.hash);
  const click = event => {
    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
    // A later interaction must not inherit an earlier fragment's focus correction.
    cancelPositioning();
    const summary = event.target.closest?.('summary');
    if (summary && disclosures.includes(summary.parentElement)) {
      pending = windowRef.requestAnimationFrame(() => {
        // Closing a tall preceding panel can move the activated heading offscreen.
        const rect = summary.getBoundingClientRect();
        if (rect.bottom <= 0 || rect.top >= windowRef.innerHeight) summary.scrollIntoView({ block: 'nearest', behavior: 'instant' });
      });
      return;
    }
    const link = event.target.closest?.('a[href]');
    if (!link || link.hasAttribute('download') || (link.target && link.target !== '_self')) return;
    const url = new URL(link.href, documentRef.baseURI);
    const here = new URL(windowRef.location.href);
    if (url.origin === here.origin && url.pathname === here.pathname && url.search === here.search && url.hash) reveal(url.hash);
  };
  const toggle = event => {
    const details = event.target;
    if (!disclosures.includes(details) || details.open) return;
    const summary = details.querySelector('summary');
    if (details.contains(documentRef.activeElement) && !summary.contains(documentRef.activeElement)) summary.focus({ preventScroll: true });
  };
  documentRef.addEventListener('click', click);
  documentRef.addEventListener('toggle', toggle, true);
  windowRef.addEventListener('hashchange', hashchange);
  hashchange();
  return { dispose() {
    cancelPositioning();
    documentRef.removeEventListener('click', click);
    documentRef.removeEventListener('toggle', toggle, true);
    windowRef.removeEventListener('hashchange', hashchange);
  } };
}
