import { useCallback, useEffect, useMemo, useState } from 'react';
import { ActivityLog } from './components/ActivityLog';
import { ActivityView } from './components/ActivityView';
import { CounterGauge } from './components/CounterGauge';
import { DevicePanel } from './components/DevicePanel';
import { ModelExplorer } from './components/ModelExplorer';
import { ResetPanel } from './components/ResetPanel';
import { SettingsView } from './components/SettingsView';
import { Sidebar, type View } from './components/Sidebar';
import { TopBar } from './components/TopBar';
import { RefreshIcon, UsbIcon } from './components/Icons';
import { TOTAL_MODELS } from './data/models';
import { usePrinterSession } from './hooks/usePrinterSession';
import { useFormatters, useI18n } from './i18n';
import type { AppSettings } from './types/settings';

function download(filename: string, payload: unknown) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], {
    type: 'application/json',
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

interface BenchProps {
  settings: AppSettings;
  onSettingsChange: (settings: AppSettings) => void;
}

export function Bench({ settings, onSettingsChange }: BenchProps) {
  const { t } = useI18n();
  const { relative } = useFormatters();
  const session = usePrinterSession();
  const [view, setView] = useState<View>('dashboard');
  const [navOpen, setNavOpen] = useState(false);

  const {
    devices,
    selected,
    counters,
    connection,
    logs,
    history,
    resetProgress,
    isRefreshing,
    isScanning,
    selectDevice,
    scan,
    refresh,
    toggleConnection,
    runReset,
    selfTest,
    clearResetProgress,
    clearHistory,
  } = session;

  useEffect(() => {
    if (!navOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setNavOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [navOpen]);

  useEffect(() => {
    window.scrollTo({ top: 0, behavior: 'auto' });
  }, [view]);

  const exportActivity = useCallback(() => {
    download(`inkreset-activity-${new Date().toISOString().slice(0, 10)}.json`, {
      generatedAt: new Date().toISOString(),
      device: selected
        ? {
            model: selected.model,
            serial: selected.serial,
            port: selected.port,
            firmware: selected.firmware,
          }
        : null,
      counters,
      history,
      logs,
    });
  }, [counters, history, logs, selected]);

  const worstCounter = useMemo(
    () => Math.max(counters.mainPad, counters.borderlessPad, counters.inkAbsorber),
    [counters],
  );

  const headline = useMemo(() => {
    if (worstCounter >= 85) return t('dash.headlineCritical');
    if (worstCounter >= 65) return t('dash.headlineWarning');
    return t('dash.headlineGood');
  }, [t, worstCounter]);

  return (
    <div className="app-shell">
      <Sidebar
        view={view}
        connection={connection}
        activityCount={logs.length}
        isOpen={navOpen}
        onNavigate={setView}
        onClose={() => setNavOpen(false)}
      />

      <div className="app-main">
        <TopBar
          selected={selected}
          connection={connection}
          isScanning={isScanning}
          onMenu={() => setNavOpen(true)}
          onScan={scan}
          onToggleConnection={toggleConnection}
        />

        <main className="app-content" id="main-content">
          {view === 'dashboard' ? (
            !selected ? (
              <section className="view-stack">
                <article className="panel empty-device">
                  <div className="empty-device-icon">
                    <UsbIcon size={30} />
                  </div>
                  <p className="eyebrow">{t('empty.eyebrow')}</p>
                  <h1>
                    {connection === 'scanning' ? t('empty.searching') : t('empty.none')}
                  </h1>
                  <p className="lede">{t('empty.lede')}</p>
                  <button
                    type="button"
                    className="button button-primary"
                    onClick={scan}
                    disabled={isScanning}
                  >
                    <RefreshIcon size={16} />
                    {isScanning ? t('common.scanning') : t('empty.scanAgain')}
                  </button>
                </article>
              </section>
            ) : (
              <section className="view-stack">
                <header className="bench-header">
                  <div className="bench-heading">
                    <p className="eyebrow">
                      {t('dash.eyebrow')}
                      <span className="eyebrow-slash">/</span>
                      {selected.series}
                    </p>
                    <h1>{t('dash.title')}</h1>
                    <p className="lede">{t('dash.lede')}</p>
                  </div>
                  <div className="bench-stats">
                    <div className="bench-stat">
                      <span className="bench-stat-label">{t('dash.state')}</span>
                      <strong
                        className={`bench-stat-value tone-text-${worstCounter >= 85 ? 'critical' : worstCounter >= 65 ? 'warning' : 'good'}`}
                      >
                        {headline}
                      </strong>
                    </div>
                    <div className="bench-stat">
                      <span className="bench-stat-label">{t('dash.resetsSession')}</span>
                      <strong className="bench-stat-value mono">
                        {String(history.length).padStart(2, '0')}
                      </strong>
                    </div>
                    <div className="bench-stat">
                      <span className="bench-stat-label">{t('dash.indexedModels')}</span>
                      <strong className="bench-stat-value mono">{TOTAL_MODELS}</strong>
                    </div>
                  </div>
                </header>

                <div className="bench-grid">
                  <article className="panel counters-panel">
                    <div className="panel-topline">
                      <span className="panel-kicker">{t('dash.counters')}</span>
                      <button
                        type="button"
                        className="text-link"
                        onClick={refresh}
                        disabled={connection !== 'online' || isRefreshing}
                      >
                        <RefreshIcon size={15} />
                        {isRefreshing
                          ? t('common.reading')
                          : t('dash.readAt', { when: relative(counters.readAt) })}
                      </button>
                    </div>

                    <div className="counters-body">
                      <CounterGauge
                        label={t('dash.mainPad')}
                        value={counters.mainPad}
                        caption={t('dash.mainPadCaption')}
                        footNote={t('dash.mainPadFoot')}
                      />
                      <div className="counters-side">
                        <CounterGauge
                          label={t('dash.borderlessPad')}
                          value={counters.borderlessPad}
                          size="sm"
                          caption={t('dash.used')}
                          footNote={t('dash.borderlessPadFoot')}
                        />
                        <CounterGauge
                          label={t('dash.inkAbsorber')}
                          value={counters.inkAbsorber}
                          size="sm"
                          caption={t('dash.used')}
                          footNote={t('dash.inkAbsorberFoot')}
                        />
                      </div>
                    </div>

                    <div className="counters-foot">
                      <div>
                        <span>{t('dash.threshold')}</span>
                        <strong>{t('dash.thresholdValue')}</strong>
                      </div>
                      <div>
                        <span>{t('dash.readVia')}</span>
                        <strong className="mono">{selected.port} / USB</strong>
                      </div>
                      <div>
                        <span>{t('dash.lastReset')}</span>
                        <strong>{selected.lastReset ?? t('time.never')}</strong>
                      </div>
                    </div>
                  </article>

                  <DevicePanel
                    device={selected}
                    devices={devices}
                    counters={counters}
                    connection={connection}
                    isRefreshing={isRefreshing}
                    onRefresh={refresh}
                    onSelectDevice={selectDevice}
                    onOpenModels={() => setView('models')}
                  />
                </div>

                <div className="bench-lower">
                  <ResetPanel
                    device={selected}
                    counters={counters}
                    connection={connection}
                    progress={resetProgress}
                    onRunReset={runReset}
                    onSelfTest={selfTest}
                    onClearProgress={clearResetProgress}
                  />
                  <ActivityLog
                    logs={logs}
                    limit={5}
                    compact
                    onViewAll={() => setView('activity')}
                  />
                </div>
              </section>
            )
          ) : null}

          {view === 'models' ? <ModelExplorer /> : null}

          {view === 'activity' ? (
            <ActivityView
              logs={logs}
              history={history}
              onExport={exportActivity}
              onClearHistory={clearHistory}
            />
          ) : null}

          {view === 'settings' ? (
            <SettingsView
              settings={settings}
              logCount={logs.length}
              historyCount={history.length}
              onChange={onSettingsChange}
              onExport={exportActivity}
              onClearHistory={clearHistory}
            />
          ) : null}
        </main>

        <footer className="app-foot">
          <span>{t('dash.footerTool')}</span>
          <span className="app-foot-dot" aria-hidden="true">
            ·
          </span>
          <span>{t('dash.footerUsb')}</span>
          <span className="app-foot-spacer" />
          <span className="mono">{selected?.model ?? t('common.noDevice')}</span>
        </footer>
      </div>
    </div>
  );
}
