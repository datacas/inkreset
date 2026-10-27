import { useFormatters, useI18n } from '../i18n';
import type {
  ConnectionState,
  PrinterDevice,
  WasteCounters,
} from '../types/printer';
import {
  AlertIcon,
  ArrowRightIcon,
  ChevronDownIcon,
  PrinterIcon,
  RefreshIcon,
  UsbIcon,
} from './Icons';

interface DevicePanelProps {
  device: PrinterDevice;
  devices: PrinterDevice[];
  counters: WasteCounters;
  connection: ConnectionState;
  isRefreshing: boolean;
  onRefresh: () => void;
  onSelectDevice: (id: string) => void;
  onOpenModels: () => void;
}

export function DevicePanel({
  device,
  devices,
  counters,
  connection,
  isRefreshing,
  onRefresh,
  onSelectDevice,
  onOpenModels,
}: DevicePanelProps) {
  const { t } = useI18n();
  const { relative } = useFormatters();
  const online = connection === 'online';
  const stateLabel = online
    ? t('device.live')
    : connection === 'scanning'
      ? t('device.enumerating')
      : t('device.offline');

  return (
    <article className="panel device-panel">
      <div className="panel-topline">
        <span className="panel-kicker">{t('device.kicker')}</span>
        <span className={`inline-status ${online ? 'is-online' : ''}`}>
          <span className="status-dot" />
          {stateLabel}
        </span>
      </div>

      <div className="device-heading">
        <div className="device-icon">
          <PrinterIcon size={30} />
          <span className="device-icon-corner" />
        </div>
        <div className="device-title">
          <p className="eyebrow">
            {device.series} / {t('device.target')}
          </p>
          <h2>{device.model}</h2>
          <p className="device-serial">{t('device.serial', { serial: device.serial })}</p>
        </div>
      </div>

      <div className="device-select-wrap">
        <label htmlFor="device-select">{t('device.active')}</label>
        <div className="select-shell">
          <select
            id="device-select"
            value={device.id}
            onChange={(event) => onSelectDevice(event.target.value)}
            className="device-select"
          >
            {devices.map((item) => (
              <option key={item.id} value={item.id}>
                {item.model} · {item.port}
              </option>
            ))}
          </select>
          <ChevronDownIcon size={17} />
        </div>
      </div>

      <dl className="spec-grid">
        <div>
          <dt>{t('device.transport')}</dt>
          <dd>
            <UsbIcon size={14} />
            {device.transport}
          </dd>
        </div>
        <div>
          <dt>{t('device.port')}</dt>
          <dd className="mono">{device.port}</dd>
        </div>
        <div>
          <dt>{t('device.firmware')}</dt>
          <dd className="mono">{device.firmware}</dd>
        </div>
        <div>
          <dt>{t('device.driver')}</dt>
          <dd>{device.driver}</dd>
        </div>
        <div>
          <dt>{t('device.lastRead')}</dt>
          <dd>{relative(counters.readAt)}</dd>
        </div>
        <div>
          <dt>{t('device.lastReset')}</dt>
          <dd>{device.lastReset ?? t('time.never')}</dd>
        </div>
      </dl>

      <div
        className="signal-line"
        aria-label={t('device.pathAria', { model: device.model })}
      >
        <span className="signal-node">{t('device.host')}</span>
        <span className="signal-track">
          <span className={`signal-pulse ${online ? 'is-live' : ''}`} />
        </span>
        <span className="signal-node mono">{device.port}</span>
        <span className="signal-track">
          <span className={`signal-pulse ${online ? 'is-live' : ''}`} />
        </span>
        <span className="signal-node">{t('device.printer')}</span>
      </div>

      {!online ? (
        <div className="inline-warning">
          <AlertIcon size={17} />
          <span>{t('device.offlineWarning')}</span>
        </div>
      ) : null}

      <div className="panel-actions">
        <button
          type="button"
          className="button button-quiet"
          onClick={onRefresh}
          disabled={!online || isRefreshing}
        >
          <RefreshIcon size={16} />
          {isRefreshing ? t('common.reading') : t('device.refresh')}
        </button>
        <button type="button" className="text-link" onClick={onOpenModels}>
          {t('device.changeModel')}
          <ArrowRightIcon size={15} />
        </button>
      </div>
    </article>
  );
}
