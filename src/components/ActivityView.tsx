import { useMemo, useState } from 'react';
import { useFormatters, useI18n, type MessageKey } from '../i18n';
import type { LogEntry, ResetRecord } from '../types/printer';
import { DownloadIcon, HistoryIcon } from './Icons';

type Filter = 'all' | LogEntry['kind'];

const FILTERS: Array<{ id: Filter; label: MessageKey }> = [
  { id: 'all', label: 'activity.filterAll' },
  { id: 'reset', label: 'activity.filterReset' },
  { id: 'connection', label: 'activity.filterConnection' },
  { id: 'warning', label: 'activity.filterWarning' },
  { id: 'system', label: 'activity.filterSystem' },
];

const KIND_KEY: Record<LogEntry['kind'], MessageKey> = {
  reset: 'kind.reset',
  connection: 'kind.connection',
  warning: 'kind.warning',
  system: 'kind.system',
};

interface ActivityViewProps {
  logs: LogEntry[];
  history: ResetRecord[];
  onExport: () => void;
  onClearHistory: () => void;
}

export function ActivityView({ logs, history, onExport, onClearHistory }: ActivityViewProps) {
  const { t } = useI18n();
  const { clock, day } = useFormatters();
  const [filter, setFilter] = useState<Filter>('all');
  const filtered = useMemo(
    () => (filter === 'all' ? logs : logs.filter((entry) => entry.kind === filter)),
    [filter, logs],
  );

  return (
    <section className="view-stack">
      <header className="page-head">
        <div>
          <p className="eyebrow">{t('activity.eyebrow')}</p>
          <h1>{t('activity.title')}</h1>
          <p className="lede">{t('activity.lede')}</p>
        </div>
        <button type="button" className="button button-quiet" onClick={onExport}>
          <DownloadIcon size={16} />
          {t('activity.export')}
        </button>
      </header>

      <div className="activity-layout">
        <article className="panel activity-main">
          <div className="panel-topline">
            <span className="panel-kicker">{t('activity.stream')}</span>
            <span className="mono log-count">
              {t('activity.shown', { count: filtered.length })}
            </span>
          </div>
          <div className="segmented" role="group" aria-label={t('activity.filterAria')}>
            {FILTERS.map((item) => (
              <button
                key={item.id}
                type="button"
                className={filter === item.id ? 'is-active' : ''}
                onClick={() => setFilter(item.id)}
                aria-pressed={filter === item.id}
              >
                {t(item.label)}
              </button>
            ))}
          </div>

          {filtered.length === 0 ? (
            <div className="empty-state">
              <HistoryIcon size={26} />
              <strong>{t('activity.nothingTitle')}</strong>
              <span>{t('activity.nothingBody')}</span>
            </div>
          ) : (
            <ol className="log-list log-list-full">
              {filtered.map((entry) => (
                <li key={entry.id} className={`log-entry kind-${entry.kind}`}>
                  <span className="log-glyph">
                    <span className="log-bullet" />
                  </span>
                  <div className="log-copy">
                    <div className="log-title-row">
                      <strong>{entry.message}</strong>
                      <time dateTime={entry.at}>{clock(entry.at)}</time>
                    </div>
                    {entry.detail ? <p>{entry.detail}</p> : null}
                    <span className="log-kind">{t(KIND_KEY[entry.kind])}</span>
                  </div>
                </li>
              ))}
            </ol>
          )}
        </article>

        <article className="panel history-panel">
          <div className="panel-topline">
            <span className="panel-kicker">{t('activity.historyKicker')}</span>
            <button type="button" className="text-link danger-link" onClick={onClearHistory}>
              {t('common.clear')}
            </button>
          </div>
          <h2>{t('activity.counterWrites')}</h2>
          <p className="panel-sub">{t('activity.historySub')}</p>

          {history.length === 0 ? (
            <div className="empty-state">
              <HistoryIcon size={24} />
              <strong>{t('activity.noResets')}</strong>
              <span>{t('activity.noResetsBody')}</span>
            </div>
          ) : (
            <ul className="history-list">
              {history.map((record) => (
                <li key={record.id}>
                  <div className="history-top">
                    <strong>{record.model}</strong>
                    <span className="mono">
                      {record.before}% → {record.after}%
                    </span>
                  </div>
                  <div className="history-meta">
                    <span>{day(record.at)}</span>
                    <span className="mono">{record.serial}</span>
                  </div>
                  <span className="history-serial mono">{record.serial}</span>
                </li>
              ))}
            </ul>
          )}
        </article>
      </div>
    </section>
  );
}
