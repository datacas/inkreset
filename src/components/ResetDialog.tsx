import { useEffect, useRef, useState } from 'react';
import { useI18n, type MessageKey } from '../i18n';
import type { PrinterDevice, ResetProgress } from '../types/printer';
import { AlertIcon, CheckIcon, ShieldIcon, XIcon } from './Icons';

interface ResetDialogProps {
  open: boolean;
  device: PrinterDevice;
  before: number;
  progress: ResetProgress;
  agreed: boolean;
  onAgreed: (value: boolean) => void;
  onConfirm: (confirm: string) => void;
  onClose: () => void;
}

const STEPS: Array<{ id: string; label: MessageKey }> = [
  { id: 'handshake', label: 'dialog.stepHandshake' },
  { id: 'writing', label: 'dialog.stepWrite' },
  { id: 'verify', label: 'dialog.stepVerify' },
];

const stageOrder = ['handshake', 'writing', 'verify', 'complete'];

export function ResetDialog({
  open,
  device,
  before,
  progress,
  agreed,
  onAgreed,
  onConfirm,
  onClose,
}: ResetDialogProps) {
  const { t } = useI18n();
  const closeRef = useRef<HTMLButtonElement>(null);
  const [typed, setTyped] = useState('');
  const busy = ['handshake', 'writing', 'verify'].includes(progress.stage);
  const complete = progress.stage === 'complete';
  const failed = progress.stage === 'error';
  const currentStageIndex = stageOrder.indexOf(progress.stage);
  const idle = progress.stage === 'idle';
  const progressLabel = idle ? t('dialog.awaiting') : progress.label;
  const canConfirm = agreed && typed.trim() === device.model && !busy;

  useEffect(() => {
    if (!open) return;
    setTyped('');
    const previous = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    closeRef.current?.focus({ preventScroll: true });

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !busy) onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      document.body.style.overflow = previous;
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [busy, onClose, open]);

  if (!open) return null;

  return (
    <div className="dialog-backdrop" role="presentation">
      <div
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="reset-dialog-title"
        aria-describedby="reset-dialog-description"
      >
        <div className="dialog-head">
          <div className={`dialog-icon ${complete ? 'is-complete' : ''}`}>
            {complete ? <CheckIcon size={24} /> : <ShieldIcon size={24} />}
          </div>
          <button
            ref={closeRef}
            type="button"
            className="icon-button"
            onClick={onClose}
            disabled={busy}
            aria-label={t('dialog.closeAria')}
          >
            <XIcon size={19} />
          </button>
        </div>

        <div className="dialog-title">
          <p className="eyebrow">{t('dialog.eyebrow')}</p>
          <h2 id="reset-dialog-title">
            {complete ? t('dialog.completeTitle') : t('dialog.title')}
          </h2>
          <p id="reset-dialog-description">
            {complete
              ? t('dialog.completeBody', { model: device.model })
              : t('dialog.body', { before })}
          </p>
        </div>

        <div className="dialog-key-row">
          <span>{t('dialog.target')}</span>
          <strong>{device.model}</strong>
          <span className="mono">{device.port}</span>
        </div>
        <div className="dialog-key-row">
          <span>{t('dialog.profile')}</span>
          <strong className="mono">{device.serial}</strong>
          <span className="key-state">
            <ShieldIcon size={14} />
            {t('dialog.verified')}
          </span>
        </div>

        {!complete && !failed ? (
          <>
            <div className="warning-block">
              <AlertIcon size={20} />
              <div>
                <strong>{t('dialog.warningTitle')}</strong>
                <p>{t('dialog.warningBody')}</p>
              </div>
            </div>

            <label className="check-row">
              <input
                type="checkbox"
                checked={agreed}
                onChange={(event) => onAgreed(event.target.checked)}
                disabled={busy}
              />
              <span className="fake-check" aria-hidden="true">
                <CheckIcon size={13} />
              </span>
              <span>{t('dialog.agree')}</span>
            </label>

            <label className="bridge-reset-label" htmlFor="reset-confirm">
              {t('dialog.typeToConfirm', { model: device.model })}
            </label>
            <input
              id="reset-confirm"
              className="bridge-reset-input"
              value={typed}
              onChange={(event) => setTyped(event.target.value)}
              spellCheck={false}
              autoComplete="off"
              placeholder={device.model}
              disabled={busy}
            />
          </>
        ) : null}

        <div className="progress-block" aria-live="polite">
          <div className="progress-head">
            <span>{progressLabel}</span>
            <span className="mono">{idle ? 0 : progress.percent}%</span>
          </div>
          <div className="progress-track">
            <span
              className={`progress-fill ${failed ? 'is-error' : ''} ${complete ? 'is-complete' : ''}`}
              style={{ width: `${Math.max(progress.percent, failed ? 100 : 0)}%` }}
            />
          </div>
          <ol className="step-list">
            {STEPS.map((step, index) => {
              const state =
                complete || index < currentStageIndex
                  ? 'done'
                  : index === currentStageIndex
                    ? 'active'
                    : 'idle';
              return (
                <li key={step.id} className={`step is-${state}`}>
                  <span className="step-mark">
                    {state === 'done' ? <CheckIcon size={11} /> : index + 1}
                  </span>
                  {t(step.label)}
                </li>
              );
            })}
          </ol>
        </div>

        {failed ? (
          <div className="dialog-error" role="alert">
            <AlertIcon size={18} />
            <span>{progress.label}</span>
          </div>
        ) : null}

        <div className="dialog-actions">
          <button
            type="button"
            className="button button-quiet"
            onClick={onClose}
            disabled={busy}
          >
            {complete ? t('common.close') : t('common.cancel')}
          </button>
          {!complete ? (
            <button
              type="button"
              className="button button-primary"
              onClick={() => onConfirm(typed.trim())}
              disabled={!canConfirm}
            >
              {busy
                ? t('dialog.writing')
                : failed
                  ? t('dialog.retry')
                  : t('dialog.resetCounter')}
            </button>
          ) : (
            <button type="button" className="button button-primary" onClick={onClose}>
              {t('dialog.returnToBench')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
