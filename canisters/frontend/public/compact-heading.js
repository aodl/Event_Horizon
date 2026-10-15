const ELIGIBLE = '(max-width: 81.25rem) and (min-aspect-ratio: 6/5)';

export function initCompactHeading(documentRef, windowRef) {
  const title = documentRef.getElementById('hero-title');
  const mark = documentRef.querySelector('.hero__compact-mark');
  if (!title || !mark) return { dispose() {} };
  let disposed = false, frame = null, observer, resize;
  const hide = () => {
    mark.classList.remove('hero__compact-mark--visible');
    if (documentRef.activeElement === mark) mark.blur();
  };
  hide();
  const media = windowRef.matchMedia?.(ELIGIBLE);
  // Shorter hero padding and variable counter height can reveal the heading even
  // in eligible landscape layouts (also after zoom/reflow). Both brands must never
  // appear together: show EVH only while the actual heading is wholly below view.
  // Without a working viewport observer fail closed, rather than guessing geometry.
  if (!media || !windowRef.IntersectionObserver || !windowRef.requestAnimationFrame) return { dispose: hide };
  const update = () => {
    frame = null;
    if (disposed) return;
    const rect = title.getBoundingClientRect();
    const viewport = windowRef.visualViewport;
    const bottom = viewport ? viewport.offsetTop + viewport.height : windowRef.innerHeight;
    const show = media.matches && title.isConnected && rect.width > 0 && rect.height > 0
      && rect.top > bottom + 1;
    if (show) mark.classList.add('hero__compact-mark--visible');
    else hide();
  };
  const schedule = () => {
    if (disposed) return;
    hide(); // Retire a possibly stale show gate before the next layout/paint.
    if (frame === null) frame = windowRef.requestAnimationFrame(update);
  };
  try {
    observer = new windowRef.IntersectionObserver(schedule, { root: null, threshold: 0 });
    observer.observe(title);
    if (windowRef.ResizeObserver) {
      resize = new windowRef.ResizeObserver(schedule);
      resize.observe(documentRef.querySelector('.hero'));
    }
  } catch { disposed = true; observer?.disconnect(); resize?.disconnect(); hide(); return { dispose: hide }; }
  update();
  media.addEventListener('change', schedule);
  windowRef.addEventListener('resize', schedule);
  windowRef.addEventListener('scroll', schedule, { passive: true });
  windowRef.visualViewport?.addEventListener('resize', schedule);
  windowRef.visualViewport?.addEventListener('scroll', schedule);
  return { dispose() {
    disposed = true; observer.disconnect(); resize?.disconnect(); hide();
    if (frame !== null) windowRef.cancelAnimationFrame(frame);
    media.removeEventListener('change', schedule);
    windowRef.removeEventListener('resize', schedule);
    windowRef.removeEventListener('scroll', schedule);
    windowRef.visualViewport?.removeEventListener('resize', schedule);
    windowRef.visualViewport?.removeEventListener('scroll', schedule);
  } };
}
