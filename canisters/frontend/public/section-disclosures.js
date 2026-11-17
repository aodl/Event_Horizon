// Native details/name owns expansion; history records user navigation only.
export function initSectionDisclosures(documentRef, windowRef) {
  const disclosures = [...documentRef.querySelectorAll('main > section > details.section-disclosure[name="event-horizon-sections"]')];
  let pending, settling, linkNavigation, activation;
  const targetFor = hash => {
    try { return documentRef.getElementById(decodeURIComponent(hash.slice(1))); } catch { return null; }
  };
  const ownerOf = target => disclosures.find(node => node.parentElement.contains(target));
  const pushFragment = hash => {
    const url = new URL(windowRef.location.href);
    if (url.hash === hash) return;
    url.hash = hash;
    windowRef.history.pushState(windowRef.history.state, '', url);
  };
  const cancelPositioning = () => {
    windowRef.cancelAnimationFrame(pending);
    windowRef.clearTimeout?.(settling);
  };
  const reveal = (hash, focus = false, restore = false) => {
    cancelPositioning();
    activation = undefined;
    const target = targetFor(hash);
    const details = ownerOf(target);
    if (!details) {
      if (restore) for (const node of disclosures) node.open = false;
      return;
    }
    const summary = details.querySelector('summary');
    const destination = target === details.parentElement ? summary : target;
    const startFocus = documentRef.activeElement;
    const position = (forceScroll = false) => {
      const active = documentRef.activeElement;
      if (active && active !== startFocus && active !== destination && active !== documentRef.body) return;
      const rect = destination.getBoundingClientRect();
      if (forceScroll || rect.top < 0 || rect.top >= windowRef.innerHeight) destination.scrollIntoView({ block: 'start', behavior: 'instant' });
      if (focus) {
        if (destination === summary || summary.contains(target)) summary.focus({ preventScroll: true });
        else {
          // Static tabindex makes browsers focus these fragments on initial load.
          // Only keyboard activation needs the subsection as a temporary focus stop.
          if (target.matches('section.project-detail')) target.tabIndex = -1;
          if (target.matches('input, select, textarea, button, a[href], [tabindex]')) target.focus({ preventScroll: true });
        }
      }
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
            if (details.open) position(!restore);
          });
        }, duration);
      });
    });
  };
  const hashchange = () => {
    const navigation = linkNavigation;
    linkNavigation = undefined;
    reveal(windowRef.location.hash, navigation?.hash === windowRef.location.hash && navigation.keyboard);
  };
  const popstate = () => {
    // Native anchor navigation can also emit popstate; its hashchange owns focus.
    if (linkNavigation?.hash !== windowRef.location.hash) reveal(windowRef.location.hash, false, true);
  };
  const click = event => {
    if (event.defaultPrevented || event.button !== 0) return;
    // A later interaction must not inherit an earlier fragment's focus correction.
    cancelPositioning();
    linkNavigation = undefined;
    const summary = event.target.closest?.('summary');
    if (summary && disclosures.includes(summary.parentElement)) {
      if (event.isTrusted) activation = { details: summary.parentElement, event };
      pending = windowRef.requestAnimationFrame(() => {
        // Closing a tall preceding panel can move the activated heading offscreen.
        const rect = summary.getBoundingClientRect();
        if (rect.bottom <= 0 || rect.top >= windowRef.innerHeight) summary.scrollIntoView({ block: 'nearest', behavior: 'instant' });
      });
      return;
    }
    if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return;
    const link = event.target.closest?.('a[href]');
    if (!link || link.hasAttribute('download') || (link.target && link.target !== '_self')) return;
    const url = new URL(link.href, documentRef.baseURI);
    const here = new URL(windowRef.location.href);
    if (url.origin === here.origin && url.pathname === here.pathname && url.search === here.search && url.hash) {
      // Firefox keyboard clicks can have detail=1; non-pointer clicks have an empty pointerType.
      const keyboard = event.isTrusted && (event.pointerType === '' || event.detail === 0);
      if (url.hash === here.hash) {
        if (ownerOf(targetFor(url.hash))) event.preventDefault?.();
        reveal(url.hash, keyboard);
      }
      else {
        // Reveal before native fragment scrolling and cancel a prior pending summary activation.
        reveal(url.hash, keyboard);
        linkNavigation = { hash: url.hash, keyboard };
      }
    }
  };
  const toggle = event => {
    const details = event.target;
    if (!disclosures.includes(details)) return;
    // Commit the native result of a trusted summary activation. Scroll correction
    // is cancellable; the user's URL transition must not depend on its frame.
    if (activation?.details === details) {
      const clickEvent = activation.event;
      activation = undefined;
      if (!clickEvent.defaultPrevented) {
        if (details.open) pushFragment(`#${details.parentElement.id}`);
        else if (ownerOf(targetFor(windowRef.location.hash)) === details) pushFragment('');
      }
    }
    if (details.open) return;
    const summary = details.querySelector('summary');
    if (details.contains(documentRef.activeElement) && !summary.contains(documentRef.activeElement)) summary.focus({ preventScroll: true });
  };
  const focusout = event => {
    if (event.target.matches?.('section.project-detail[tabindex="-1"]')) event.target.removeAttribute('tabindex');
  };
  documentRef.addEventListener('click', click);
  documentRef.addEventListener('toggle', toggle, true);
  documentRef.addEventListener('focusout', focusout);
  windowRef.addEventListener('hashchange', hashchange);
  windowRef.addEventListener('popstate', popstate);
  reveal(windowRef.location.hash);
  return { dispose() {
    cancelPositioning();
    activation = linkNavigation = undefined;
    documentRef.removeEventListener('click', click);
    documentRef.removeEventListener('toggle', toggle, true);
    documentRef.removeEventListener('focusout', focusout);
    windowRef.removeEventListener('hashchange', hashchange);
    windowRef.removeEventListener('popstate', popstate);
  } };
}
