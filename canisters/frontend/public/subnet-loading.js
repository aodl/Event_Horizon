// The external document's load event is not a dashboard-data readiness signal.
export function initSubnetLoading(documentRef, windowRef) {
  const section = documentRef.querySelector('#subnet-map');
  const frame = section?.querySelector('iframe');
  const layer = section?.querySelector('.subnet-section__loading');
  const status = section?.querySelector('.subnet-section__status');
  if (!frame || !layer || !status) return { dispose() {} };
  let slowTimer, settleTimer;
  layer.hidden = false;
  const clearTimers = () => {
    windowRef.clearTimeout(slowTimer);
    windowRef.clearTimeout(settleTimer);
  };
  const loaded = () => {
    // No external URL means this is the initial empty document, not our request.
    // Do not inspect the cross-origin document (including its Document getter).
    if (!frame.getAttribute('src')) return;
    clearTimers();
    // A brief visual settling interval, not a claim about cross-origin data.
    settleTimer = windowRef.setTimeout(() => {
      layer.hidden = true;
      status.textContent = '';
    }, 500);
  };
  frame.addEventListener('load', loaded);
  slowTimer = windowRef.setTimeout(() => {
    // Keep failed/pending documents dark; a late load can still reveal the map.
    layer.hidden = false;
    status.textContent = 'Taking longer than expected. You can also view the subnet on ICP Dashboard using the link above.';
  }, 12000);
  return { dispose() {
    clearTimers();
    frame.removeEventListener('load', loaded);
    layer.hidden = true;
    status.textContent = '';
  } };
}
