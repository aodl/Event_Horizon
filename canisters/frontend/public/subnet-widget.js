export const SUBNET_EMBED_URL = 'https://dashboard.internetcomputer.org/embed/subnet-map?subnet=pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae';

const controllers = new WeakMap();

export function initSubnetWidget(documentRef, windowRef) {
  if (controllers.has(documentRef)) return controllers.get(documentRef);
  const node = id => documentRef.getElementById(id);
  const launcher = node('subnet-widget'), dialog = node('subnet-dialog');
  const expand = node('subnet-expand'), dismiss = node('subnet-dismiss');
  const minimise = node('subnet-minimise'), close = node('subnet-close');
  const frame = node('subnet-frame'), fallback = node('subnet-fallback');
  const main = node('main-content');
  // The existing fixed skip link becomes visible on focus, even when the
  // main landmark is below the hero or above the current scroll position.
  const returnFocus = documentRef.querySelector?.('.skip-link') ?? main;
  if (![launcher, dialog, expand, dismiss, minimise, close, frame, fallback, main].every(Boolean)
      || typeof dialog.showModal !== 'function') return { dispose() {} };

  let state = 'collapsed', disposed = false, scroll = null;
  const root = documentRef.documentElement, body = documentRef.body;
  const lock = () => {
    scroll = { x: windowRef.scrollX, y: windowRef.scrollY, top: body.style.top };
    body.style.top = `${-scroll.y}px`;
    root.classList.add('subnet-map-open');
  };
  const unlock = () => {
    if (!scroll) return;
    body.style.top = scroll.top;
    root.classList.remove('subnet-map-open');
    windowRef.scrollTo({ left: scroll.x, top: scroll.y, behavior: 'instant' });
    scroll = null;
  };
  const render = () => {
    launcher.hidden = state !== 'collapsed';
    expand.setAttribute('aria-expanded', String(state === 'expanded'));
  };
  const leave = (next, restoreFocus = true) => {
    if (disposed || state === 'dismissed' || state === next) return;
    state = next;
    // Set the destination first: close events are queued and may arrive after
    // another opening. Never briefly restore the launcher on dismissal.
    render();
    frame.removeAttribute('src');
    if (dialog.open) dialog.close();
    unlock();
    if (restoreFocus) (state === 'collapsed' ? expand : returnFocus).focus({ preventScroll: true });
  };
  const open = () => {
    if (disposed || state !== 'collapsed') return;
    lock();
    try {
      dialog.showModal();
    } catch {
      unlock();
      // Keep a conventional working link if native modal opening fails.
      expand.hidden = true; fallback.hidden = false;
      fallback.focus({ preventScroll: true });
      return;
    }
    state = 'expanded'; render();
    frame.src = SUBNET_EMBED_URL;
    minimise.focus({ preventScroll: true });
  };
  const collapse = () => leave('collapsed');
  const hide = () => leave('dismissed');
  const cancel = event => { event.preventDefault(); collapse(); };
  const closed = () => { if (!dialog.open && state === 'expanded') collapse(); };
  // Release the external document even on a back/forward-cache navigation.
  const pagehide = () => { if (state === 'expanded') leave('collapsed', false); };
  const listeners = [
    [expand, 'click', open], [dismiss, 'click', hide],
    [minimise, 'click', collapse], [close, 'click', hide],
    [dialog, 'cancel', cancel], [dialog, 'close', closed],
    [windowRef, 'pagehide', pagehide],
  ];
  for (const [target, type, handler] of listeners) target.addEventListener(type, handler);
  fallback.hidden = true; expand.hidden = false; dismiss.hidden = false;
  render();
  const controller = { dispose() {
    if (disposed) return;
    const ownsFocus = launcher.contains(documentRef.activeElement) || dialog.contains(documentRef.activeElement);
    leave('dismissed', ownsFocus);
    disposed = true;
    for (const [target, type, handler] of listeners) target.removeEventListener(type, handler);
  } };
  controllers.set(documentRef, controller);
  return controller;
}
