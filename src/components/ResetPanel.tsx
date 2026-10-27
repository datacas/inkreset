import { useState } from 'react';
import { useI18n } from '../i18n';
import type {
  ConnectionState,
  PrinterDevice,
  ResetProgress,
  SelfTestResult,
  WasteCounters,
} from '../types/printer';
import {
  AlertIcon,
  ArrowRightIcon,
  CheckIcon,
  ShieldIcon,
  UsbIcon,
} from './Icons';
import { ResetDialog } from './ResetDialog';

interface ResetPanelProps {
  device: PrinterDevice;
  counters: WasteCounters;
  connection: ConnectionState;
  progress: ResetProgress;
  onRunReset: (confirm: string) => Promise<WasteCounters>;
  onSelfTest: () => Promise<SelfTestResult>;
  onClearProgress: () => void;
}

export function ResetPanel({
  device,
  counters,
  connection,
  progress,
  onRunReset,
  onSelfTest,
  onClearProgress,
}: ResetPanelProps) {
  const { t } = useI18n();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [agreed, setAgreed] = useState(false);
  const [testing, setTesting] = useState(false);
  const [test, setTest] = useState<SelfTestResult | null>(null);
  const [testError, setTestError] = useState('');

  const online = connection === 'online';
  const complete = progress.stage === 'complete';
  const busy = ['handshake', 'writing', 'verify'].includes(progress.stage);

  const closeDialog = () => {
    if (busy) return;
    setDialogOpen(false);
    setAgreed(false);
    if (complete || progress.stage === 'error') onClearProgress();
  };

  const confirmReset = async (confirm: string) => {
    try {
      await onRunReset(confirm);
    } catch {
      // The hook exposes the error through progress.label.
    }
  };

  const runSelfTest = async () => {
    setTesting(true);
    setTest(null);
    setTestError('');
    try {
      setTest(await onSelfTest());
    } catch (error) {
      setTestError(error instanceof Error ? error.message : String(error));
    } finally {
      setTesting(false);
    }
  };

  return (
    <>
      <article className={`panel reset-panel ${complete ? 'is-complete' : ''}`}>
        <div className="panel-topline">
          <span className="panel-kicker">{t('reset.kicker')}</span>
          <span className={`inline-status ${online ? 'is-online' : ''}`}>
            <span className="status-dot" />
            {online ? t('reset.verified') : t('reset.usbOffline')}
          </span>
        </div>

        <div className="reset-header">
          <div className="reset-icon">
            {complete ? <CheckIcon size={24} /> : <ShieldIcon size={24} />}
          </div>
          <div>
            <h2>{complete ? t('reset.doneTitle') : t('reset.title')}</h2>
            <p>{complete ? t('reset.doneBody') : t('reset.body')}</p>
          </div>
        </div>

        {complete ? (
          <div className="reset-success" role="status">
            <span className="success-mark">
              <CheckIcon size={19} />
            </span>
            <div>
              <strong>{t('reset.recorded')}</strong>
              <p>
                {t('reset.recordedBody', {
                  main: counters.mainPad,
                  borderless: counters.borderlessPad,
                })}
              </p>
            </div>
            <button
              type="button"
              className="text-link"
              onClick={() => {
                onClearProgress();
              }}
            >
              {t('reset.another')}
              <ArrowRightIcon size={15} />
            </button>
          </div>
        ) : (
          <>
            <div className="reset-facts">
              <div>
                <span>{t('reset.printer')}</span>
                <strong>{device.model}</strong>
              </div>
              <div>
                <span>{t('reset.currentMain')}</span>
                <strong>{counters.mainPad}%</strong>
              </div>
              <div>
                <span>{t('reset.transport')}</span>
                <strong className="mono">{device.port}</strong>
              </div>
            </div>

            <div className="safety-note">
              <ShieldIcon size={18} />
              <p>
                <strong>{t('reset.padFirst')}</strong> {t('reset.padFirstBody')}
              </p>
            </div>

            <div className="reset-test">
              <button
                type="button"
                className="button button-quiet"
                onClick={() => void runSelfTest()}
                disabled={!online || busy || testing}
              >
                {testing ? t('reset.testRunning') : t('reset.testWrite')}
              </button>
              {test ? (
                <p className={`reset-test-result ${test.unchanged ? 'is-ok' : 'is-bad'}`}>
                  {test.unchanged ? <CheckIcon size={14} /> : <AlertIcon size={14} />}
                  <span>
                    {test.unchanged
                      ? t('reset.testOk', {
                          address: test.address,
                          value: `0x${test.before.toString(16).toUpperCase().padStart(2, '0')}`,
                        })
                      : t('reset.testBad', { before: test.before, after: test.after })}
                  </span>
                </p>
              ) : null}
              {testError ? (
                <p className="reset-test-result is-bad">
                  <AlertIcon size={14} />
                  <span>{testError}</span>
                </p>
              ) : null}
            </div>

            <button
              type="button"
              className="button button-primary reset-submit"
              onClick={() => {
                setAgreed(false);
                setDialogOpen(true);
              }}
              disabled={!online || busy}
            >
              <UsbIcon size={17} />
              {t('reset.continue')}
              <ArrowRightIcon size={16} />
            </button>

            <p className="reset-legal">{t('reset.legal')}</p>
          </>
        )}
      </article>

      <ResetDialog
        open={dialogOpen}
        device={device}
        before={counters.mainPad}
        progress={progress}
        agreed={agreed}
        onAgreed={setAgreed}
        onConfirm={confirmReset}
        onClose={closeDialog}
      />
    </>
  );
}
