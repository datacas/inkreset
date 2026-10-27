import type { MessageKey } from '../i18n';
import { useI18n } from '../i18n';
import type { ConnectionState } from '../types/printer';
import {
  CodeIcon,
  GaugeIcon,
  GridIcon,
  HistoryIcon,
  LogoMark,
  SettingsIcon,
  UsbIcon,
} from './Icons';

export type View = 'dashboard' | 'models' | 'activity' | 'settings';

interface SidebarProps {
  view: View;
  connection: ConnectionState;
  activityCount: number;
  isOpen: boolean;
  onNavigate: (view: View) => void;
  onClose: () => void;
}

const navItems: Array<{
  id: View;
  label: MessageKey;
  hint: MessageKey;
  icon: typeof GaugeIcon;
}> = [
  { id: 'dashboard', label: 'nav.dashboard', hint: 'nav.dashboardHint', icon: GaugeIcon },
  { id: 'models', label: 'nav.models', hint: 'nav.modelsHint', icon: GridIcon },
  { id: 'activity', label: 'nav.activity', hint: 'nav.activityHint', icon: HistoryIcon },
  { id: 'settings', label: 'nav.settings', hint: 'nav.settingsHint', icon: SettingsIcon },
];

export function Sidebar({
  view,
  connection,
  activityCount,
  isOpen,
  onNavigate,
  onClose,
}: SidebarProps) {
  const { t } = useI18n();
  const online = connection === 'online';

  return (
    <>
      <button
        className={`sidebar-scrim ${isOpen ? 'is-open' : ''}`}
        type="button"
        aria-label={t('aria.closeNavigation')}
        onClick={onClose}
      />
      <aside
        className={`sidebar ${isOpen ? 'is-open' : ''}`}
        aria-label={t('aria.primaryNavigation')}
      >
        <div className="sidebar-brand">
          <LogoMark />
          <div>
            <p className="brand-name">INKRESET</p>
            <p className="brand-sub"></p>
          </div>
          <span className="brand-build">TS</span>
        </div>

        <div className="sidebar-section-label">{t('sidebar.workspace')}</div>
        <nav className="sidebar-nav">
          {navItems.map(({ id, label, hint, icon: Icon }) => (
            <button
              key={id}
              type="button"
              className={`nav-item ${view === id ? 'is-active' : ''}`}
              onClick={() => {
                onNavigate(id);
                onClose();
              }}
              aria-current={view === id ? 'page' : undefined}
            >
              <span className="nav-icon">
                <Icon size={19} />
              </span>
              <span className="nav-copy">
                <span className="nav-label">{t(label)}</span>
                <span className="nav-hint">{t(hint)}</span>
              </span>
              {id === 'activity' && activityCount > 0 ? (
                <span className="nav-count">{Math.min(activityCount, 99)}</span>
              ) : null}
            </button>
          ))}
        </nav>

        <div className="sidebar-spacer" />

        <div className="transport-card">
          <div className="transport-head">
            <span className={`status-dot ${online ? 'is-online' : ''}`} />
            <span>{online ? t('sidebar.transportLive') : t('sidebar.transportIdle')}</span>
            <UsbIcon size={16} />
          </div>
          <p>{t('sidebar.transportNote')}</p>
          <div className="transport-meta">
            <span>{t('sidebar.protocol')}</span>
            <strong>USB 2.0</strong>
          </div>
          <div className="transport-meta">
            <span>{t('sidebar.build')}</span>
            <strong>0.2.0 / TS</strong>
          </div>
        </div>

        <button
          type="button"
          className="sidebar-foot"
          onClick={() => {
            onNavigate('settings');
            onClose();
          }}
        >
          <CodeIcon size={17} />
          <span>{t('sidebar.openSource')}</span>
          <span className="foot-arrow">↗</span>
        </button>
      </aside>
    </>
  );
}
