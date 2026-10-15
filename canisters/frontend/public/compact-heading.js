const BOTTOM_PEEK_PX = 128;
// Match the art-first compact layout, not the old landscape-only shortcut.
// An aspect-ratio veto would make the heading allowance ineffective on square
// or taller windows, even when only a few pixels of the title are visible.
const ELIGIBLE = '(max-width: 81.25rem), (max-aspect-ratio: 1/1)';

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
  // The compact brand fits the dark corner while the large title follows the art.
  // A partial bottom-edge view need not replace the compact brand immediately.
  // Keep a bounded allowance rather than reacting to the first visible pixels.
  // Hide EVH once more than 128px (or three quarters of a small heading) enters
  // view. Actual geometry still matters on tall/zoomed screens.
  // Without a working viewport observer fail closed, rather than guessing geometry.
  if (!media || !windowRef.IntersectionObserver || !windowRef.requestAnimationFrame) return { dispose: hide };
  const update = () => {
    frame = null;
    if (disposed) return;
    const rect = title.getBoundingClientRect();
    const viewport = windowRef.visualViewport;
    const bottom = viewport ? viewport.offsetTop + viewport.height : windowRef.innerHeight;
    const peek = Math.min(BOTTOM_PEEK_PX, rect.height * .75);
    const show = media.matches && title.isConnected && rect.width > 0 && rect.height > 0
      && rect.top >= Math.max(viewport?.offsetTop ?? 0, bottom - peek);
    if (show) mark.classList.add('hero__compact-mark--visible');
    else hide();
  };
  const schedule = () => {
    if (disposed) return;
    if (frame === null) frame = windowRef.requestAnimationFrame(update);
  };
  // Do not hide/show the focused anchor on every scroll event: that can interrupt
  // its native smooth-scroll navigation. Coalesce scroll geometry; invalidate the
  // gate immediately for layout/media changes that could expose a substantial second title.
  const invalidate = () => { if (!disposed) { hide(); schedule(); } };
  try {
    observer = new windowRef.IntersectionObserver(schedule, { root: null, threshold: 0 });
    observer.observe(title);
    if (windowRef.ResizeObserver) {
      resize = new windowRef.ResizeObserver(invalidate);
      resize.observe(documentRef.querySelector('.hero'));
    }
  } catch { disposed = true; observer?.disconnect(); resize?.disconnect(); hide(); return { dispose: hide }; }
  update();
  media.addEventListener('change', invalidate);
  windowRef.addEventListener('resize', invalidate);
  windowRef.addEventListener('scroll', schedule, { passive: true });
  windowRef.visualViewport?.addEventListener('resize', invalidate);
  windowRef.visualViewport?.addEventListener('scroll', schedule);
  return { dispose() {
    disposed = true; observer.disconnect(); resize?.disconnect(); hide();
    if (frame !== null) windowRef.cancelAnimationFrame(frame);
    media.removeEventListener('change', invalidate);
    windowRef.removeEventListener('resize', invalidate);
    windowRef.removeEventListener('scroll', schedule);
    windowRef.visualViewport?.removeEventListener('resize', invalidate);
    windowRef.visualViewport?.removeEventListener('scroll', schedule);
  } };
}
