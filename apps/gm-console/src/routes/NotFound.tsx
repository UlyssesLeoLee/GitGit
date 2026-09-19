import { Link } from 'react-router-dom';
import { EmptyState } from '@/components';
import { useTranslate } from '@/i18n/runtime';

/** Fallback route when nothing else matches. */
export function NotFound() {
  const t = useTranslate();
  return (
    <EmptyState
      title={t('errors.notFoundTitle')}
      description={t('errors.notFoundDescription')}
      icon="404"
      action={
        <Link to="/" className="btn-primary">
          {t('errors.goHome')}
        </Link>
      }
    />
  );
}