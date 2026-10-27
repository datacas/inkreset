import { useI18n } from '../i18n';
import type { AppSettings } from '../types/settings';
import { HardwareBridgePanel } from './HardwareBridgePanel';
import {
  AlertIcon,
  CheckIcon,
  CodeIcon,
  DownloadIcon,
  InfoIcon,
  ShieldIcon,
  UsbIcon,
} from './Icons';

interface SettingsViewProps {
  settings: AppSettings;
  logCount: number;
  historyCount: number;
  onChange: (settings: AppSettings) => void;
  onExport: () => void;
  onClearHistory: () => void;
}

function Toggle({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="setting-row">
      <span className="setting-copy">
        <strong>{label}</strong>
        <span>{description}</span>
      </span>
      <input
        type="checkbox"
        className="sr-only"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span className={`switch ${checked ? 'is-on' : ''}`} aria-hidden="true">
        <span />
      </span>
    </label>
  );
}

export function SettingsView({
  settings,
  logCount,
  historyCount,
  onChange,
  onExport,
  onClearHistory,
}: SettingsViewProps) {
  const { t } = useI18n();

  return (
    <section className="view-stack">
      <header className="page-head">
        <div>
          <p className="eyebrow">{t('settings.eyebrow')}</p>
          <h1>{t('settings.title')}</h1>
          <p className="lede">{t('settings.lede')}</p>
        </div>
      </header>

      <div className="settings-grid">
        <article className="panel settings-panel">
          <div className="panel-topline">
            <span className="panel-kicker">{t('settings.interface')}</span>
            <span className="mono">{t('settings.local')}</span>
          </div>
          <h2>{t('settings.behaviour')}</h2>
          <p className="panel-sub">{t('settings.behaviourSub')}</p>

          <div className="setting-row">
            <span className="setting-copy">
              <strong>{t('settings.language')}</strong>
              <span>{t('settings.languageSub')}</span>
            </span>
            <div className="segmented" role="group" aria-label={t('settings.language')}>
              <button
                type="button"
                className={settings.language === 'es' ? 'is-active' : ''}
                onClick={() => onChange({ ...settings, language: 'es' })}
                aria-pressed={settings.language === 'es'}
              >
                {t('settings.spanish')}
              </button>
              <button
                type="button"
                className={settings.language === 'en' ? 'is-active' : ''}
                onClick={() => onChange({ ...settings, language: 'en' })}
                aria-pressed={settings.language === 'en'}
              >
                {t('settings.english')}
              </button>
            </div>
          </div>

          <Toggle
            label={t('settings.reduceMotion')}
            description={t('settings.reduceMotionSub')}
            checked={settings.reduceMotion}
            onChange={(value) => onChange({ ...settings, reduceMotion: value })}
          />
          <Toggle
            label={t('settings.compact')}
            description={t('settings.compactSub')}
            checked={settings.compact}
            onChange={(value) => onChange({ ...settings, compact: value })}
          />

          <div className="setting-row is-locked">
            <span className="setting-copy">
              <strong>{t('settings.network')}</strong>
              <span>{t('settings.networkSub')}</span>
            </span>
            <span className="lock-tag">
              <UsbIcon size={13} />
              {t('settings.usbOnly')}
            </span>
          </div>
        </article>

        <article className="panel settings-panel">
          <div className="panel-topline">
            <span className="panel-kicker">{t('settings.data')}</span>
            <span className="mono">JSON</span>
          </div>
          <h2>{t('settings.records')}</h2>
          <p className="panel-sub">{t('settings.recordsSub')}</p>

          <div className="data-stats">
            <div>
              <span className="data-number">{logCount}</span>
              <span>{t('settings.sessionEvents')}</span>
            </div>
            <div>
              <span className="data-number">{historyCount}</span>
              <span>{t('settings.savedResets')}</span>
            </div>
          </div>

          <div className="settings-actions">
            <button type="button" className="button button-quiet" onClick={onExport}>
              <DownloadIcon size={16} />
              {t('settings.exportActivity')}
            </button>
            <button
              type="button"
              className="button button-danger"
              onClick={onClearHistory}
              disabled={historyCount === 0}
            >
              {t('settings.clearHistory')}
            </button>
          </div>
        </article>

        <HardwareBridgePanel />

        <article className="panel settings-panel about-panel">
          <div className="panel-topline">
            <span className="panel-kicker">{t('settings.aboutKicker')}</span>
            <span className="mono">v0.2.0</span>
          </div>
          <div className="about-brand">
            <CodeIcon size={22} />
            <div>
              <h2>InkReset</h2>
              <p>{t('settings.aboutBody')}</p>
            </div>
          </div>
          <div className="about-facts">
            <div>
              <span>{t('settings.stack')}</span>
              <strong>React · TypeScript · Rust · Tauri</strong>
            </div>
            <div>
              <span>{t('settings.transport')}</span>
              <strong>USBPRINT + IEEE 1284.4 (native)</strong>
            </div>
            <div>
              <span>{t('settings.scope')}</span>
              <strong>{t('settings.scopeValue')}</strong>
            </div>
          </div>
          <div className="about-notes">
            <p>
              <ShieldIcon size={16} />
              {t('settings.aboutLicence')}
            </p>
            <p>
              <AlertIcon size={16} />
              {t('settings.aboutTested')}
            </p>
            <p>
              <AlertIcon size={16} />
              {t('settings.aboutPad')}
            </p>
            <p>
              <CheckIcon size={16} />
              {t('settings.aboutKey')}
            </p>
          </div>
          <div className="about-foot">
            <InfoIcon size={15} />
            <span>{t('settings.aboutFoot')}</span>
          </div>
        </article>
      </div>
    </section>
  );
}
