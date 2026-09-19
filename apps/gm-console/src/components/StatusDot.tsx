/**
 * Small inline dot — green for online, amber for degraded, red for
 * offline. Decorative; the surrounding text carries the meaning.
 */
interface StatusDotProps {
  state: 'online' | 'degraded' | 'offline' | 'unknown';
  label?: string;
}

const STATE_CLASSES: Record<StatusDotProps['state'], string> = {
  online: 'bg-emerald-500',
  degraded: 'bg-amber-500',
  offline: 'bg-rose-500',
  unknown: 'bg-slate-400',
};

export function StatusDot({ state, label }: StatusDotProps) {
  return (
    <span className="inline-flex items-center gap-2 text-sm">
      <span
        aria-hidden="true"
        className={'inline-block h-2 w-2 rounded-full ' + STATE_CLASSES[state]}
      />
      {label ? <span>{label}</span> : null}
    </span>
  );
}