export function debounce<Arguments extends unknown[]>(
  func: (...args: Arguments) => void,
  wait: number
): ((...args: Arguments) => void) & { cancel: () => void } {
  let timeout: number | null = null;

  const debounced = (...args: Arguments) => {
    if (timeout) window.clearTimeout(timeout);
    timeout = window.setTimeout(() => {
      func(...args);
      timeout = null;
    }, wait);
  };

  debounced.cancel = () => {
    if (timeout) {
      window.clearTimeout(timeout);
      timeout = null;
    }
  };

  return debounced;
}
