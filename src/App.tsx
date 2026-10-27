import { useEffect, useState } from 'react';
import { Bench } from './Bench';
import { I18nProvider } from './i18n';
import { loadSettings, saveSettings, type AppSettings } from './types/settings';

export default function App() {
  const [settings, setSettings] = useState<AppSettings>(loadSettings);

  useEffect(() => {
    saveSettings(settings);
    document.documentElement.classList.toggle('reduce-motion', settings.reduceMotion);
    document.documentElement.classList.toggle('compact-rows', settings.compact);
    document.documentElement.lang = settings.language;
  }, [settings]);

  return (
    <I18nProvider language={settings.language}>
      <Bench settings={settings} onSettingsChange={setSettings} />
    </I18nProvider>
  );
}
