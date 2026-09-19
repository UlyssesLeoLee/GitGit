import { useEffect, useRef, useState } from 'react';
import { copyToClipboard } from '@/lib/clipboard';
import { useToastsStore } from '@/stores/toasts';

interface CopyButtonProps {
  text: string;
  label?: string;
  copiedLabel?: string;
  /** Toast message; if omitted, no toast is shown. */
  toastMessage?: string;
  className?: string;
}

/**
 * Copy-to-clipboard button. Visually flips to a "copied" state for
 * `RESET_MS` after success, then snaps back. Errors fall back to a
 * toast with `error` severity.
 */
const RESET_MS = 1400;

export function CopyButton({
  text,
  label = 'Copy',
  copiedLabel = 'Copied',
  toastMessage,
  className,
}: CopyButtonProps) {
  const [done, setDone] = useState(false);
  const timerRef = useRef<number | null>(null);
  const pushToast = useToastsStore((s) => s.push);

  useEffect(
    () => () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    },
    [],
  );

  const handleClick = async (): Promise<void> => {
    const ok = await copyToClipboard(text);
    if (!ok) {
      pushToast('error', 'Copy failed — your browser blocked clipboard access');
      return;
    }
    setDone(true);
    if (toastMessage) pushToast('success', toastMessage);
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(() => setDone(false), RESET_MS);
  };

  return (
    <button
      type="button"
      onClick={handleClick}
      aria-live="polite"
      aria-label={done ? copiedLabel : label}
      className={
        'btn-secondary ' +
        (done ? 'border-emerald-500 text-emerald-600 dark:text-emerald-400 ' : '') +
        (className ?? '')
      }
    >
      {done ? copiedLabel : label}
    </button>
  );
}