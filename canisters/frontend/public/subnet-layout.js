const BASE_WIDTH = 760;
const BASE_HEIGHT = 520;

export function subnetGeometry(width) {
  if (!Number.isFinite(width) || width <= 0) return null;
  const scale = Math.max(1, width / BASE_WIDTH);
  return { scaled: width > BASE_WIDTH, scale, height: BASE_HEIGHT * scale };
}

// Observe only the parent content width. Never inspect or navigate the external frame.
export function initSubnetLayout(documentRef, windowRef) {
  const embed = documentRef.querySelector('.subnet-section__embed');
  if (!embed?.querySelector('iframe') || !windowRef.ResizeObserver) return { dispose() {} };
  const widget = documentRef.querySelector('#subnet-map');
  let observer, disposed = false, previousWidth = 0;
  const reset = () => {
    embed.removeAttribute('data-scaled');
    embed.style.removeProperty('--subnet-scale');
    embed.style.removeProperty('--subnet-height');
  };
  const dispose = () => {
    disposed = true;
    observer?.disconnect();
    reset();
  };
  const update = width => {
    if (disposed) return;
    if (widget?.dataset?.mode === 'compact') { reset(); previousWidth = 0; return; }
    if ((Math.abs(width - previousWidth) < .25 && (width > BASE_WIDTH) === (previousWidth > BASE_WIDTH))) return;
    const geometry = subnetGeometry(width);
    if (!geometry) return;
    previousWidth = width;
    if (geometry.scaled) {
      embed.style.setProperty('--subnet-scale', String(geometry.scale));
      embed.style.setProperty('--subnet-height', `${geometry.height}px`);
      // Opt in only after both geometry values are available.
      embed.setAttribute('data-scaled', '');
    } else reset();
  };
  try {
    observer = new windowRef.ResizeObserver(entries => {
      try { update(entries[0].contentRect.width); } catch { dispose(); }
    });
    observer.observe(embed);
    update(embed.getBoundingClientRect().width);
  } catch { dispose(); }
  return { dispose, update() {
    previousWidth = 0;
    update(embed.getBoundingClientRect().width);
  } };
}
