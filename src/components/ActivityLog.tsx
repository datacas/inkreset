import { useFormatters, useI18n } from '../i18n';
import type { LogEntry } from '../types/printer';
import { AlertIcon, CheckIcon, HistoryIcon, InfoIcon, UsbIcon } from './Icons';

interface ActivityLogProps {
  logs: LogEntry[];
  limit?: number;
  onViewAll?: () => void;
  compact?: boolean;
}

function LogGlyph({ kind }: { kind: LogEntry['kind'] }) {
  if (kind === 'reset') return <CheckIcon size={15} />;
  if (kind === 'warning') return <AlertIcon size={15} />;
  if (kind === 'connection') return <UsbIcon size={15} />;
  return <InfoIcon size={15} />;
}

export function ActivityLog({ logs, limit, onViewAll, compact = false }: ActivityLogProps) {
  const { t } = useI18n();
  const { clock, relative } = useFormatters();
  const visible = typeof limit === 'number' ? logs.slice(0, limit) : logs;

  return (
    <article className={`panel log-panel ${compact ? 'is-compact' : ''}`}>
      <div className="panel-topline">
        <span className="panel-kicker">{t('log.kicker')}</span>
        <span className="log-count mono">
          {t(logs.length === 1 ? 'log.eventsOne' : 'log.eventsMany', {
            count: logs.length,
          })}
        </span>
      </div>

      <div className="log-header">
        <div>
          <h2>{t('log.title')}</h2>
          <p>{t('log.body')}</p>
        </div>
        {onViewAll ? (
          <button type="button" className="text-link" onClick={onViewAll}>
            {t('log.viewAll')}
            <HistoryIcon size={15} />
          </button>
        ) : null}
      </div>

      {visible.length === 0 ? (
        <div className="empty-state">
          <HistoryIcon size={26} />
          <strong>{t('log.emptyTitle')}</strong>
          <span>{t('log.emptyBody')}</span>
        </div>
      ) : (
        <ol className="log-list">
          {visible.map((entry) => (
            <li key={entry.id} className={`log-entry kind-${entry.kind}`}>
              <span className="log-glyph">
                <LogGlyph kind={entry.kind} />
              </span>
              <div className="log-copy">
                <div className="log-title-row">
                  <strong>{entry.message}</strong>
                  <time dateTime={entry.at} title={entry.at}>
                    {compact ? relative(entry.at) : clock(entry.at)}
                  </time>
                </div>
                {entry.detail ? <p>{entry.detail}</p> : null}
              </div>
            </li>
          ))}
        </ol>
      )}
    </article>
  );
}
