import { initSubnetLayout } from './subnet-layout.js';
import { initSubnetLoading } from './subnet-loading.js';

// The dialog and its frame stay mounted across native non-modal/modal switching.
export function initSubnetWidget(documentRef, windowRef) {
  const dialog = documentRef.querySelector('#subnet-map');
  if (!dialog?.show || !dialog.showModal) return { dispose() {} };
  const frame = dialog.querySelector('iframe');
  const embed = dialog.querySelector('.subnet-section__embed');
  const expand = dialog.querySelector('.subnet-expand');
  const minimise = dialog.querySelector('.subnet-minimise');
  const dismiss = dialog.querySelector('.subnet-dismiss');
  let dismissed = false, returnFocus, loading;
  const layout = initSubnetLayout(documentRef, windowRef);
  const restoreFocus = fallback => {
    const destination = returnFocus?.isConnected && returnFocus !== documentRef.body && !dialog.contains(returnFocus) ? returnFocus : fallback;
    destination?.focus({ preventScroll: true });
    if (documentRef.activeElement !== destination) fallback?.focus({ preventScroll: true });
  };
  const present = expanded => {
    if (dismissed || (dialog.open && dialog.dataset.mode === (expanded ? 'expanded' : 'compact'))) return;
    if (expanded) returnFocus = documentRef.activeElement;
    // Close before changing mode: showModal cannot promote an open non-modal dialog.
    dialog.close();
    dialog.dataset.mode = expanded ? 'expanded' : 'compact';
    embed.inert = !expanded;
    embed.setAttribute('aria-hidden', String(!expanded));
    frame.tabIndex = expanded ? 0 : -1;
    if (expanded) dialog.showModal();
    else dialog.show();
    layout.update?.();
    if (expanded) minimise.focus({ preventScroll: true });
    else restoreFocus(expand);
  };
  const open = () => present(true);
  const shrink = () => present(false);
  const remove = () => {
    if (dismissed) return;
    const focused = dialog.contains(documentRef.activeElement);
    dismissed = true;
    windowRef.clearTimeout(startTimer);
    loading?.dispose();
    layout.dispose();
    dialog.close();
    frame.remove(); // Release the external application only on permanent dismissal.
    if (focused) restoreFocus(documentRef.querySelector('main > section summary'));
  };
  const cancel = event => { event.preventDefault(); shrink(); };
  const hashchange = () => { if (windowRef.location.hash === '#subnet-map') open(); };
  expand.addEventListener('click', open);
  minimise.addEventListener('click', shrink);
  dismiss.addEventListener('click', remove);
  dialog.addEventListener('cancel', cancel);
  windowRef.addEventListener('hashchange', hashchange);
  // Schedule after the main app has started its independent queries and rendering.
  const startTimer = windowRef.setTimeout(() => {
    if (dismissed) return;
    loading = initSubnetLoading(documentRef, windowRef);
    frame.src = frame.dataset.src;
  }, 0);
  // Native non-modal presentation without taking the page's initial focus.
  dialog.open = true;
  hashchange();
  return { dispose() {
    remove();
    expand.removeEventListener('click', open);
    minimise.removeEventListener('click', shrink);
    dismiss.removeEventListener('click', remove);
    dialog.removeEventListener('cancel', cancel);
    windowRef.removeEventListener('hashchange', hashchange);
  } };
}
