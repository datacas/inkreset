import { useCallback, useEffect, useMemo, useState } from 'react';
import { useI18n, type Translate } from '../i18n';
import {
  bridgeIsNative,
  listUsbPrinters,
  readBridgeStatus,
  readWasteCounters,
  resetWasteCounters,
  type BridgeStatus,
  type CounterReport,
  type UsbPrinterInfo,
} from '../services/bridge/tauriBridge';
import { AlertIcon, CheckIcon, CopyIcon, RefreshIcon, UsbIcon } from './Icons';

const identity = (printer: UsbPrinterInfo, t: Translate) =>
  printer.model ?? printer.friendlyName ?? t('bridge.unknownPrinter');

const vidPid = (printer: UsbPrinterInfo, t: Translate) =>
  [printer.vendorId, printer.productId].filter(Boolean).join(':') ||
  t('bridge.vidUnknown');

const hex = (value: number, width = 2) => value.toString(16).toUpperCase().padStart(width, '0');

interface ResetDraft {
  path: string;
  model: string;
}

export function HardwareBridgePanel() {
  const { t } = useI18n();
  const native = bridgeIsNative();
  const [status, setStatus] = useState<BridgeStatus | null>(null);
  const [printers, setPrinters] = useState<UsbPrinterInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);

  const [reports, setReports] = useState<Record<string, CounterReport>>({});
  const [pending, setPending] = useState<string | null>(null);
  const [actionError, setActionError] = useState<Record<string, string>>({});
  const [resetDraft, setResetDraft] = useState<ResetDraft | null>(null);
  const [confirmText, setConfirmText] = useState('');
  const [resetResult, setResetResult] = useState<Record<string, string>>({});

  const scan = useCallback(async () => {
    if (!native) return;
    setBusy(true);
    setError(null);
    try {
      const nextStatus = await readBridgeStatus();
      setStatus(nextStatus);
      const found = await listUsbPrinters();
      setPrinters(found);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }, [native]);

  useEffect(() => {
    void scan();
  }, [scan]);

  const copy = async (value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(value);
      window.setTimeout(() => setCopied(null), 1600);
    } catch {
      setCopied(null);
    }
  };

  const readCounters = async (printer: UsbPrinterInfo) => {
    if (!printer.model) return;
    setPending(printer.path);
    setActionError((current) => ({ ...current, [printer.path]: '' }));
    try {
      const report = await readWasteCounters(printer.path, printer.model);
      setReports((current) => ({ ...current, [printer.path]: report }));
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setActionError((current) => ({ ...current, [printer.path]: message }));
    } finally {
      setPending(null);
    }
  };

  const startReset = (printer: UsbPrinterInfo) => {
    if (!printer.model) return;
    setResetDraft({ path: printer.path, model: printer.model });
    setConfirmText('');
  };

  const cancelReset = () => {
    setResetDraft(null);
    setConfirmText('');
  };

  const submitReset = async () => {
    if (!resetDraft) return;
    const { path, model } = resetDraft;
    setPending(path);
    setActionError((current) => ({ ...current, [path]: '' }));
    try {
      const result = await resetWasteCounters(path, model, confirmText.trim());
      setResetResult((current) => ({
        ...current,
        [path]: t('bridge.wrote', { bytes: result.bytesWritten }),
      }));
      const report = await readWasteCounters(path, model);
      setReports((current) => ({ ...current, [path]: report }));
      setResetDraft(null);
      setConfirmText('');
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setActionError((current) => ({ ...current, [path]: message }));
    } finally {
      setPending(null);
    }
  };

  const countersAvailable = status?.counterProtocol ?? false;
  const printerCount = useMemo(() => printers.length, [printers]);

  return (
    <article className="panel settings-panel bridge-panel">
      <div className="panel-topline">
        <span className="panel-kicker">{t('bridge.kicker')}</span>
        <span className={`mono ${native ? 'bridge-tag is-live' : 'bridge-tag'}`}>
          {native ? t('bridge.native') : t('bridge.noBridge')}
        </span>
      </div>
      <h2>{t('bridge.title')}</h2>
      <p className="panel-sub">{t('bridge.body')}</p>

      {!native && (
        <div className="bridge-callout">
          <UsbIcon size={18} />
          <div>
            <strong>{t('bridge.notLoaded')}</strong>
            <p>
              {t('bridge.notLoadedBody')}{' '}
              <code className="bridge-cmd">npm run tauri:dev</code>
            </p>
          </div>
        </div>
      )}

      {native && (
        <>
          <div className="bridge-toolbar">
            <span className="bridge-state">
              <span className={`dot ${busy ? 'is-busy' : 'is-live'}`} />
              {busy
                ? t('bridge.scanningInterfaces')
                : t(printerCount === 1 ? 'bridge.interfaceOne' : 'bridge.interfaceMany', {
                    count: printerCount,
                  })}
            </span>
            <div className="bridge-actions">
              {status?.profiles ? (
                <span className="bridge-profiles">
                  {t('bridge.profiles', { count: status.profiles })}
                </span>
              ) : null}
              <button
                type="button"
                className="button button-quiet"
                onClick={() => void scan()}
                disabled={busy}
              >
                <RefreshIcon size={15} />
                {t('bridge.rescan')}
              </button>
            </div>
          </div>

          {status && <p className="bridge-detail">{status.detail}</p>}

          {error && (
            <p className="bridge-error">
              <AlertIcon size={16} />
              {error}
            </p>
          )}

          {!busy && printers.length === 0 && !error && (
            <p className="bridge-empty">{t('bridge.empty')}</p>
          )}

          <ul className="bridge-list">
            {printers.map((printer) => {
              const report = reports[printer.path];
              const message = actionError[printer.path];
              const confirmation = resetResult[printer.path];
              const working = pending === printer.path;
              const drafting = resetDraft?.path === printer.path;
              const canConfirm =
                Boolean(resetDraft) &&
                confirmText.trim() === resetDraft?.model &&
                confirmText.trim().length > 0;

              return (
                <li key={printer.path} className="bridge-item">
                  <div className="bridge-item-head">
                    <span className="bridge-item-icon">
                      <UsbIcon size={16} />
                    </span>
                    <div>
                      <strong>{identity(printer, t)}</strong>
                      <span className="mono">
                        {vidPid(printer, t)}
                        {printer.serial ? ` · ${printer.serial}` : ''}
                      </span>
                    </div>
                    <span className="bridge-live">
                      <CheckIcon size={13} />
                      {t('bridge.live')}
                    </span>
                  </div>

                  <p className="bridge-instance">{printer.instanceId || printer.friendlyName}</p>

                  {printer.ieee1284Id ? (
                    <div className="bridge-id">
                      <code>{printer.ieee1284Id}</code>
                      <button
                        type="button"
                        className="icon-button"
                        aria-label={t('bridge.copyId')}
                        onClick={() => void copy(printer.ieee1284Id ?? '')}
                      >
                        {copied === printer.ieee1284Id ? (
                          <CheckIcon size={14} />
                        ) : (
                          <CopyIcon size={14} />
                        )}
                      </button>
                    </div>
                  ) : (
                    <p className="bridge-warn">
                      <AlertIcon size={14} />
                      {t('bridge.noId')}
                    </p>
                  )}

                  {countersAvailable && (
                    <div className="bridge-counter-block">
                      <div className="bridge-counter-head">
                        <span className="bridge-section-label">
                          {t('bridge.wasteCounters')}
                        </span>
                        <div className="bridge-actions">
                          <button
                            type="button"
                            className="button button-quiet"
                            onClick={() => void readCounters(printer)}
                            disabled={working || !printer.model}
                          >
                            {working
                              ? t('bridge.talking')
                              : report
                                ? t('bridge.reRead')
                                : t('bridge.readCounters')}
                          </button>
                          {report && report.counters.length > 0 && (
                            <button
                              type="button"
                              className="button button-ghost-danger"
                              onClick={() => startReset(printer)}
                              disabled={working}
                            >
                              {t('bridge.resetAction')}
                            </button>
                          )}
                        </div>
                      </div>

                      {!printer.model && (
                        <p className="bridge-warn">
                          <AlertIcon size={14} />
                          {t('bridge.noModel')}
                        </p>
                      )}

                      {message && (
                        <p className="bridge-error">
                          <AlertIcon size={16} />
                          {message}
                        </p>
                      )}

                      {confirmation && (
                        <p className="bridge-ok">
                          <CheckIcon size={14} />
                          {confirmation}
                        </p>
                      )}

                      {report && (
                        <>
                          <div className="bridge-chain">
                            <span className="bridge-section-label">{t('bridge.profile')}</span>
                            {report.chain.map((key) => (
                              <span key={key} className="bridge-chip mono">
                                {key}
                              </span>
                            ))}
                            <span className="bridge-chip mono">
                              {t('bridge.factory')}{' '}
                              {report.factory.map((byte) => hex(byte)).join(' ')}
                            </span>
                            <span className="bridge-chip mono">
                              {t('bridge.socket')} {hex(report.socket)} · ch{' '}
                              {report.sendSize}/{report.recvSize}
                            </span>
                          </div>

                          <ul className="bridge-counters">
                            {report.counters.map((counter) => {
                              const percent = Math.max(
                                0,
                                Math.min(100, Math.round(counter.percent)),
                              );
                              return (
                                <li key={counter.index} className="bridge-counter">
                                  <div className="bridge-counter-row">
                                    <span>
                                      {t('bridge.counter', { n: counter.index + 1 })}
                                    </span>
                                    <span className="mono">
                                      {counter.value} / {counter.max}
                                    </span>
                                  </div>
                                  <div className="bridge-meter" aria-hidden="true">
                                    <span style={{ width: `${percent}%` }} />
                                  </div>
                                  <code className="bridge-regs">
                                    {t('bridge.registers')}{' '}
                                    {counter.registers.map((r) => hex(r, 4)).join(' ')}
                                  </code>
                                </li>
                              );
                            })}
                          </ul>

                          {report.counters.length === 0 && (
                            <p className="bridge-warn">
                              <AlertIcon size={14} />
                              {t('bridge.noCounters')}
                            </p>
                          )}

                          <p className="bridge-note">{report.note}</p>
                        </>
                      )}

                      {drafting && (
                        <div className="bridge-reset">
                          <p className="bridge-reset-title">
                            <AlertIcon size={16} />
                            {t('bridge.resetTitle')}
                          </p>
                          <p className="bridge-reset-body">{t('bridge.resetBody')}</p>
                          <label className="bridge-reset-label">
                            {t('bridge.typeConfirm', { model: resetDraft?.model ?? '' })}
                          </label>
                          <input
                            className="bridge-reset-input"
                            value={confirmText}
                            onChange={(event) => setConfirmText(event.target.value)}
                            spellCheck={false}
                            autoComplete="off"
                            placeholder={resetDraft?.model}
                          />
                          <div className="bridge-actions bridge-reset-actions">
                            <button
                              type="button"
                              className="button button-quiet"
                              onClick={cancelReset}
                              disabled={working}
                            >
                              {t('common.cancel')}
                            </button>
                            <button
                              type="button"
                              className="button button-danger"
                              onClick={() => void submitReset()}
                              disabled={!canConfirm || working}
                            >
                              {working ? t('bridge.writing') : t('bridge.writeTable')}
                            </button>
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </li>
              );
            })}
          </ul>
        </>
      )}
    </article>
  );
}
