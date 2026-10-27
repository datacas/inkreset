import type { Language } from '../i18n/messages';

export interface AppSettings {
  language: Language;
  reduceMotion: boolean;
  compact: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  language: 'es',
  reduceMotion: false,
  compact: false,
};

const SETTINGS_KEY = 'inkreset:settings';

const isLanguage = (value: unknown): value is Language => value === 'es' || value === 'en';

export function loadSettings(): AppSettings {
  try {
    const raw = window.localStorage.getItem(SETTINGS_KEY);
    if (!raw) return DEFAULT_SETTINGS;
    const parsed = JSON.parse(raw) as Partial<AppSettings>;
    return {
      language: isLanguage(parsed.language) ? parsed.language : DEFAULT_SETTINGS.language,
      reduceMotion: Boolean(parsed.reduceMotion),
      compact: Boolean(parsed.compact),
    };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(settings: AppSettings): void {
  try {
    window.localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // Preferences are optional; the console works without persistence.
  }
}
