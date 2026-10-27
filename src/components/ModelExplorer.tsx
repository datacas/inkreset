import { useDeferredValue, useEffect, useMemo, useState } from 'react';
import { MODEL_ROWS, TOTAL_MODELS } from '../data/models';
import { useI18n } from '../i18n';
import {
  bridgeIsNative,
  listDeviceProfiles,
  type ProfileSummary,
} from '../services/bridge/tauriBridge';
import { CheckIcon, SearchIcon, UsbIcon } from './Icons';

const SERIES = ['All', ...new Set(MODEL_ROWS.map((row) => row.series))];

const FLAT_MODELS = MODEL_ROWS.flatMap((row) =>
  row.models.map((model) => ({
    model,
    series: row.series,
    note: row.note,
  })),
);

const shortChain = (chain: string[]) =>
  chain.length <= 4 ? chain : [...chain.slice(0, 3), `+${chain.length - 3}`];

export function ModelExplorer() {
  const { t } = useI18n();
  const native = bridgeIsNative();
  const [query, setQuery] = useState('');
  const [series, setSeries] = useState('All');
  const [source, setSource] = useState<'curated' | 'recovered'>('curated');
  const deferredQuery = useDeferredValue(query);

  const [recovered, setRecovered] = useState<ProfileSummary[] | null>(null);
  const [recoveredError, setRecoveredError] = useState<string | null>(null);

  useEffect(() => {
    if (source !== 'recovered' || !native) return;
    let live = true;
    setRecoveredError(null);
    listDeviceProfiles(deferredQuery.trim() || undefined)
      .then((rows) => {
        if (live) setRecovered(rows);
      })
      .catch((cause: unknown) => {
        if (!live) return;
        setRecovered([]);
        setRecoveredError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      live = false;
    };
  }, [deferredQuery, native, source]);

  const results = useMemo(() => {
    const needle = deferredQuery.trim().toLowerCase();
    return FLAT_MODELS.filter((row) => {
      const inSeries = series === 'All' || row.series === series;
      const inQuery =
        !needle ||
        row.model.toLowerCase().includes(needle) ||
        row.series.toLowerCase().includes(needle);
      return inSeries && inQuery;
    });
  }, [deferredQuery, series]);

  const visible = results.slice(0, 300);
  const recoveredRows = recovered ?? [];
  const resetCount = recoveredRows.filter(
    (row) => row.resetPairs > 0 && row.counters > 0,
  ).length;
  const readOnlyCount = recoveredRows.filter(
    (row) => row.resetPairs === 0 && row.counters > 0,
  ).length;
  const serviceOnlyCount = recoveredRows.filter((row) => row.counters === 0).length;


  return (
    <section className="view-stack">
      <header className="page-head">
        <div>
          <p className="eyebrow">{t('models.eyebrow')}</p>
          <h1>{t('models.title')}</h1>
          <p className="lede">
            {source === 'recovered'
              ? t('models.ledeRecovered')
              : t('models.ledeCurated', { total: TOTAL_MODELS })}
          </p>
        </div>
        <div className="head-stat">
          <span className="head-stat-number">
            {source === 'recovered' ? recoveredRows.length : TOTAL_MODELS}
          </span>
          <span className="head-stat-label">
            {source === 'recovered' ? t('models.profilesLoaded') : t('models.modelsIndexed')}
          </span>
        </div>
      </header>

      <div className="model-toolbar">
        <div className="search-shell">
          <SearchIcon size={18} />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t('models.searchPlaceholder')}
            aria-label={t('models.searchAria')}
          />
          <span className="search-count mono">
            {source === 'recovered' ? recoveredRows.length : results.length}
          </span>
        </div>
        {native ? (
          <div className="series-strip" role="group" aria-label={t('models.dataSource')}>
            <button
              type="button"
              className={`filter-chip ${source === 'curated' ? 'is-active' : ''}`}
              onClick={() => setSource('curated')}
              aria-pressed={source === 'curated'}
            >
              {t('models.curated')}
            </button>
            <button
              type="button"
              className={`filter-chip ${source === 'recovered' ? 'is-active' : ''}`}
              onClick={() => setSource('recovered')}
              aria-pressed={source === 'recovered'}
            >
              {t('models.recovered')}
            </button>
          </div>
        ) : null}
        {source === 'curated' ? (
          <div className="series-strip" role="group" aria-label={t('models.filterSeries')}>
            {SERIES.map((item) => (
              <button
                key={item}
                type="button"
                className={`filter-chip ${series === item ? 'is-active' : ''}`}
                onClick={() => setSeries(item)}
                aria-pressed={series === item}
              >
                {item === 'All' ? t('models.all') : item}
              </button>
            ))}
          </div>
        ) : null}
      </div>

      {source === 'recovered' ? (
        <div className="model-table-wrap panel">
          {recoveredError ? <p className="bridge-error">{recoveredError}</p> : null}
          <table className="model-table">
            <thead>
              <tr>
                <th scope="col">{t('models.colModel')}</th>
                <th scope="col">{t('models.colChain')}</th>
                <th scope="col">{t('models.colCounters')}</th>
                <th scope="col">{t('models.colReset')}</th>
              </tr>
            </thead>
            <tbody>
              {recoveredRows.map((row) => {
                const canRead = row.counters > 0;
                const canReset = row.resetPairs > 0 && canRead;
                return (
                  <tr key={`${row.key}-${row.model}`} className="model-row">
                    <td>
                      <span className="model-name">{row.title || row.model}</span>
                      <span className="model-note mono">{row.model}</span>
                    </td>
                    <td>
                      <span className="bridge-chain mono">
                        {shortChain(row.chain).map((key) => (
                          <span key={key} className="bridge-chip">
                            {key}
                          </span>
                        ))}
                      </span>
                    </td>
                    <td>
                      <span className="mono">{row.hasService ? `${row.counters}` : '—'}</span>
                    </td>
                    <td>
                      {canReset ? (
                        <span className="coverage-tag">
                          <CheckIcon size={13} />
                          {t('models.supported')}
                        </span>
                      ) : canRead ? (
                        <span className="coverage-tag is-muted">{t('models.readOnly')}</span>
                      ) : (
                        <span className="coverage-tag is-muted">{t('models.serviceOnly')}</span>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {!recovered && !recoveredError ? (
            <p className="table-footnote">{t('models.loading')}</p>
          ) : null}
          {recovered && recoveredRows.length === 0 && !recoveredError ? (
            <div className="empty-state model-empty">
              <SearchIcon size={26} />
              <strong>{t('models.noMatchRecovered')}</strong>
              <span>{t('models.tryShorter')}</span>
            </div>
          ) : null}
          {recoveredRows.length > 0 ? (
            <p className="table-footnote">
              {t('models.footnoteRecovered', {
                heading: t('models.recoveredCount', { count: recoveredRows.length }),
                reset: resetCount,
                readOnly: readOnlyCount,
                serviceOnly: serviceOnlyCount,
              })}
            </p>
          ) : null}
        </div>
      ) : (
        <div className="model-table-wrap panel">
          <table className="model-table">
            <thead>
              <tr>
                <th scope="col">{t('models.colModel')}</th>
                <th scope="col">{t('models.colSeries')}</th>
                <th scope="col">{t('models.colTransport')}</th>
                <th scope="col">{t('models.colCoverage')}</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((row, index) => {
                const firstOfSeries =
                  index === 0 || visible[index - 1].series !== row.series;
                return (
                  <tr key={`${row.series}-${row.model}`} className="model-row">
                    <td>
                      <span className="model-name">{row.model}</span>
                      {row.note && firstOfSeries ? (
                        <span className="model-note">{row.note}</span>
                      ) : null}
                    </td>
                    <td>{row.series}</td>
                    <td>
                      <span className="usb-tag">
                        <UsbIcon size={13} />
                        {t('models.usbOnly')}
                      </span>
                    </td>
                    <td>
                      <span className="coverage-tag">
                        <CheckIcon size={13} />
                        {t('models.supported')}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {visible.length === 0 ? (
            <div className="empty-state model-empty">
              <SearchIcon size={26} />
              <strong>{t('models.noMatch')}</strong>
              <span>{t('models.noMatchBody')}</span>
            </div>
          ) : null}
          {results.length > visible.length ? (
            <p className="table-footnote">
              {t('models.showing', { shown: visible.length, total: results.length })}
            </p>
          ) : null}
        </div>
      )}
    </section>
  );
}
