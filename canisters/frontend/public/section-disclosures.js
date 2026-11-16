// Native details/name owns expansion. This helper only reveals fragment targets.
export function initSectionDisclosures(documentRef, windowRef) {
  const disclosures = [...documentRef.querySelectorAll('main > section > details.section-disclosure[name="event-horizon-sections"]')];
  let pending;
  const reveal = hash => {
    windowRef.cancelAnimationFrame(pending);
    let target;
    try { target = documentRef.getElementById(decodeURIComponent(hash.slice(1))); } catch { return; }
    const details = disclosures.find(node => node.parentElement.contains(target));
    if (!details) return;
    details.open = true;
    const summary = details.querySelector('summary');
    pending = windowRef.requestAnimationFrame(() => {
      // Let normal fragment navigation scroll; correct only a still-hidden target.
      const destination = target === details.parentElement ? summary : target;
      const rect = destination.getBoundingClientRect();
      if (rect.top < 0 || rect.top >= windowRef.innerHeight) destination.scrollIntoView({ block: 'start', behavior: 'instant' });
      if (destination === summary || summary.contains(target)) summary.focus({ preventScroll: true });
      else if (target.matches('input, select, textarea, button, a[href], [tabindex]')) target.focus({ preventScroll: true });
    });
  };
  const hashchange = () => reveal(windowRef.location.hash);
  const click = event => {
    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
    const summary = event.target.closest?.('summary');
    if (summary && disclosures.includes(summary.parentElement)) {
      windowRef.cancelAnimationFrame(pending);
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
    windowRef.cancelAnimationFrame(pending);
    documentRef.removeEventListener('click', click);
    documentRef.removeEventListener('toggle', toggle, true);
    windowRef.removeEventListener('hashchange', hashchange);
  } };
}
