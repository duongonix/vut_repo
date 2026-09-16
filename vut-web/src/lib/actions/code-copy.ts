export function codeCopy(node: HTMLElement) {
  const timers = new Map<HTMLButtonElement, ReturnType<typeof setTimeout>>();
  let alive = true;
  const status = document.createElement('span');
  status.className = 'sr-only';
  status.setAttribute('role', 'status');
  node.append(status);
  async function click(event: MouseEvent) {
    if (!(event.target instanceof Element)) return;
    const button = event.target.closest<HTMLButtonElement>('[data-copy-code]');
    if (!button || !node.contains(button)) return;
    const code = button.closest('.code-block')?.querySelector('code')?.textContent;
    if (code === undefined || code === null) return;
    try {
      await navigator.clipboard.writeText(code);
      if (!alive) return;
      button.textContent = 'Copied';
    } catch {
      if (!alive) return;
      button.textContent = 'Copy failed';
    }
    status.textContent = button.textContent;
    clearTimeout(timers.get(button));
    const timer = setTimeout(() => {
      button.textContent = 'Copy';
      status.textContent = '';
      timers.delete(button);
    }, 1800);
    timers.set(button, timer);
  }
  node.addEventListener('click', click);
  return {
    destroy() {
      alive = false;
      node.removeEventListener('click', click);
      timers.forEach(clearTimeout);
      status.remove();
    }
  };
}
