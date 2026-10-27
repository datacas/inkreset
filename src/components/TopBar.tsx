import { useI18n } from '../i18n';
import type { ConnectionState, PrinterDevice } from '../types/printer';
import { MenuIcon, PowerIcon, RefreshIcon, UsbIcon } from './Icons';

interface TopBarProps {
  selected?: PrinterDevice;
  connection: ConnectionState;
  isScanning: boolean;
  onMenu: () => void;
  onScan: () => void;
  onToggleConnection: () => void;
}

export function TopBar({
  selected,
  connection,
  isScanning,
  onMenu,
  onScan,
  onToggleConnection,
}: TopBarProps) {
  const { t } = useI18n();
  const online = connection === 'online';
  const scanning = connection === 'scanning' || isScanning;
  const connectionLabel = scanning
    ? t('topbar.scanningUsb')
    : online
      ? t('topbar.usbConnected')
      : t('topbar.usbOffline');

  return (
    <header className="topbar">
      <div className="topbar-left">
        <button
          type="button"
          className="icon-button mobile-menu"
          onClick={onMenu}
          aria-label={t('aria.openNavigation')}
        >
          <MenuIcon />
        </button>
        <div className="breadcrumb">
          <span>INKRESET</span>
          <span className="breadcrumb-slash">/</span>
          <span className="breadcrumb-current">{t('topbar.breadcrumb')}</span>
        </div>
      </div>

      <div className="topbar-right">
        <div className={`link-state ${online ? 'is-online' : ''} ${scanning ? 'is-busy' : ''}`}>
          <UsbIcon size={16} />
          <span className="link-state-text">{connectionLabel}</span>
          <span className="link-state-device">{selected?.port ?? '—'}</span>
        </div>

        <button
          type="button"
          className="button button-quiet topbar-button"
          onClick={onScan}
          disabled={scanning}
        >
          <RefreshIcon size={16} />
          <span>{scanning ? t('topbar.scanning') : t('topbar.scanBus')}</span>
        </button>

        <button
          type="button"
          className={`button icon-only ${online ? 'button-ghost-danger' : 'button-primary'}`}
          onClick={onToggleConnection}
          aria-label={online ? t('topbar.disconnect') : t('topbar.reconnect')}
          title={online ? t('topbar.disconnect') : t('topbar.reconnect')}
          disabled={scanning}
        >
          <PowerIcon size={17} />
        </button>
      </div>
    </header>
  );
}
