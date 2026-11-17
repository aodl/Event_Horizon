import { initSubnetLayout } from './subnet-layout.js';
import { initSubnetLoading } from './subnet-loading.js';

// One mounted dialog/frame; data-mode owns presentation, previousMode only Escape.
export function initSubnetWidget(documentRef, windowRef) {
  const dialog = documentRef.querySelector('#subnet-map');
  if (!dialog?.show || !dialog.showModal) return { dispose() {} };
  const frame = dialog.querySelector('iframe');
  const embed = dialog.querySelector('.subnet-section__embed');
  const layout = initSubnetLayout(documentRef, windowRef);
  let previousMode = 'thumbnail', loading;
  const start = () => {
    if (loading) return;
    loading = initSubnetLoading(documentRef, windowRef);
    frame.src = frame.dataset.src;
  };
  const present = (mode, fragment = false) => {
    const current = dialog.dataset.mode;
    if (mode === current) return;
    const expanded = mode === 'expanded', modalChange = expanded || current === 'expanded';
    if (expanded) previousMode = fragment ? 'thumbnail' : current;
    // Only modal/non-modal switching requires closing the native dialog.
    if (modalChange) dialog.close();
    dialog.dataset.mode = mode;
    embed.inert = !expanded;
    embed.setAttribute('aria-hidden', String(!expanded));
    frame.tabIndex = expanded ? 0 : -1;
    if (modalChange) expanded ? dialog.showModal() : dialog.show();
    layout.update?.();
    dialog.scrollTop = 0; // Keep the destination toolbar visible in short views.
    if (mode !== 'minimised') start();
    dialog.querySelector(expanded ? '.subnet-minimise' : mode === 'minimised' ? '.subnet-tab-trigger' : '.subnet-maximise').focus({ preventScroll: true });
  };
  const click = event => {
    const control = event.target.closest?.('button[data-subnet-mode]');
    if (control && dialog.contains(control)) present(control.dataset.subnetMode);
  };
  const cancel = event => { event.preventDefault(); present(previousMode); };
  const hashchange = () => { if (windowRef.location.hash === '#subnet-map') present('expanded', true); };
  dialog.addEventListener('click', click);
  dialog.addEventListener('cancel', cancel);
  windowRef.addEventListener('hashchange', hashchange);
  // Avoid taking initial focus from the page's Skip link.
  dialog.open = true;
  hashchange();
  return { dispose() {
    loading?.dispose();
    layout.dispose();
    dialog.close();
    dialog.removeEventListener('click', click);
    dialog.removeEventListener('cancel', cancel);
    windowRef.removeEventListener('hashchange', hashchange);
  } };
}
