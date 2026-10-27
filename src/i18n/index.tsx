import { createContext, useCallback, useContext, useMemo, type ReactNode } from 'react';
import { dictionaries, type Language, type MessageKey } from './messages';

export type { Language, MessageKey };

export type Translate = (key: MessageKey, vars?: Record<string, string | number>) => string;

interface I18nValue {
  language: Language;
  locale: string;
  t: Translate;
}

const I18nContext = createContext<I18nValue | null>(null);

const interpolate = (text: string, vars?: Record<string, string | number>): string => {
  if (!vars) return text;
  return text.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in vars ? String(vars[name]) : match,
  );
};

export function I18nProvider({
  language,
  children,
}: {
  language: Language;
  children: ReactNode;
}) {
  const t = useCallback<Translate>(
    (key, vars) => {
      const table = dictionaries[language] ?? dictionaries.en;
      return interpolate(table[key] ?? dictionaries.en[key] ?? key, vars);
    },
    [language],
  );

  const value = useMemo<I18nValue>(
    () => ({
      language,
      locale: language === 'es' ? 'es-ES' : 'en-GB',
      t,
    }),
    [language, t],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  const value = useContext(I18nContext);
  if (!value) {
    throw new Error('useI18n must be used inside <I18nProvider>.');
  }
  return value;
}

/** Date and duration formatters bound to the active language. */
export function useFormatters() {
  const { locale, t } = useI18n();

  return useMemo(() => {
    const clock = new Intl.DateTimeFormat(locale, {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
    const day = new Intl.DateTimeFormat(locale, {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    });
    const relative = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });

    return {
      clock: (iso: string) => (iso ? clock.format(new Date(iso)) : '—'),
      day: (iso: string) => (iso ? day.format(new Date(iso)) : '—'),
      relative: (iso: string) => {
        if (!iso) return '—';
        const diff = Math.max(0, Date.now() - new Date(iso).getTime());
        const minutes = Math.floor(diff / 60_000);
        if (minutes < 1) return t('time.justNow');
        if (minutes < 60) return relative.format(-minutes, 'minute');
        const hours = Math.floor(minutes / 60);
        if (hours < 24) return relative.format(-hours, 'hour');
        const days = Math.floor(hours / 24);
        return relative.format(-days, 'day');
      },
    };
  }, [locale, t]);
}
