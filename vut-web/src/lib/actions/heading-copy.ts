/** Preserve normal anchor navigation, while also copying its deep link. */
export function headingCopy(node: HTMLElement) {
  const status = document.createElement('span');
  status.className = 'sr-only';
  status.setAttribute('role', 'status');
  node.append(status);
  let timer: ReturnType<typeof setTimeout>;
  let alive = true;
  async function copy(event: MouseEvent) {
    const anchor =
      event.target instanceof Element
        ? event.target.closest<HTMLAnchorElement>('a.heading-anchor')
        : null;
    if (!anchor || !node.contains(anchor)) return;
    try {
      await navigator.clipboard.writeText(anchor.href);
      if (alive) status.textContent = 'Section link copied';
    } catch {
      if (alive)
        status.textContent = 'Copy unavailable. Copy the section URL from the address bar.';
    }
    clearTimeout(timer);
    if (alive)
      timer = setTimeout(() => {
        status.textContent = '';
      }, 2000);
  }
  node.addEventListener('click', copy);
  return {
    destroy() {
      alive = false;
      clearTimeout(timer);
      node.removeEventListener('click', copy);
      status.remove();
    }
  };
}
