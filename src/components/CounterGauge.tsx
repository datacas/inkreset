import { useI18n } from '../i18n';

interface CounterGaugeProps {
  label: string;
  value: number;
  size?: 'lg' | 'sm';
  caption: string;
  footNote?: string;
}

const TONE = (value: number) => {
  if (value >= 85) return 'critical';
  if (value >= 65) return 'warning';
  return 'good';
};

const RADIUS = 52;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

export function CounterGauge({
  label,
  value,
  size = 'lg',
  caption,
  footNote,
}: CounterGaugeProps) {
  const { t } = useI18n();
  const clamped = Math.min(100, Math.max(0, value));
  const tone = TONE(clamped);
  const offset = CIRCUMFERENCE - (clamped / 100) * CIRCUMFERENCE;

  return (
    <div className={`counter-gauge counter-gauge-${size} tone-${tone}`}>
      <div
        className="gauge-ring-wrap"
        role="meter"
        aria-label={t('gauge.aria', { label, value: clamped })}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={clamped}
      >
        <svg className="gauge-ring" viewBox="0 0 120 120" aria-hidden="true">
          <circle className="gauge-track" cx="60" cy="60" r={RADIUS} />
          <circle
            className="gauge-value"
            cx="60"
            cy="60"
            r={RADIUS}
            strokeDasharray={CIRCUMFERENCE}
            strokeDashoffset={offset}
          />
          <circle className="gauge-inner-line" cx="60" cy="60" r="38" />
        </svg>
        <div className="gauge-center">
          <span className="gauge-number">
            {clamped}
            <span className="gauge-percent">%</span>
          </span>
          <span className="gauge-caption">{caption}</span>
        </div>
      </div>
      <div className="gauge-meta">
        <span className="gauge-label">{label}</span>
        {footNote ? <span className="gauge-foot">{footNote}</span> : null}
      </div>
    </div>
  );
}
