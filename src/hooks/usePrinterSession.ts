import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useI18n } from '../i18n';
import { UsbPrinterTransport } from '../services/bridge/printerTransport';
import { bridgeIsNative } from '../services/bridge/tauriBridge';
import type {
  ConnectionState,
  LogEntry,
  PrinterDevice,
  ResetProgress,
  ResetRecord,
  SelfTestResult,
  WasteCounters,
} from '../types/printer';

const HISTORY_KEY = 'inkreset:history';

const emptyCounters = (): WasteCounters => ({
  mainPad: 0,
  borderlessPad: 0,
  inkAbsorber: 0,
  readAt: '',
});

const loadHistory = (): ResetRecord[] => {
  try {
    const raw = window.localStorage.getItem(HISTORY_KEY);
    return raw ? (JSON.parse(raw) as ResetRecord[]) : [];
  } catch {
    return [];
  }
};

const createLog = (
  kind: LogEntry['kind'],
  message: string,
  detail?: string,
): LogEntry => ({
  id: globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`,
  at: new Date().toISOString(),
  kind,
  message,
  detail,
});

const messageOf = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

export function usePrinterSession() {
  const { t } = useI18n();

  // The transport is created once, but its translator must always read the
  // current language, so it goes through a ref instead of capturing `t`.
  const tRef = useRef(t);
  tRef.current = t;
  const transport = useRef(
    new UsbPrinterTransport((key, vars) => tRef.current(key, vars)),
  ).current;

  const [devices, setDevices] = useState<PrinterDevice[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [counters, setCounters] = useState<WasteCounters>(emptyCounters);
  const [connection, setConnection] = useState<ConnectionState>('scanning');
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [history, setHistory] = useState<ResetRecord[]>(loadHistory);
  const [resetProgress, setResetProgress] = useState<ResetProgress>(() => ({
    stage: 'idle',
    label: tRef.current('progress.ready'),
    percent: 0,
  }));
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isScanning, setIsScanning] = useState(false);

  const selected = useMemo(
    () => devices.find((device) => device.id === selectedId) ?? devices[0],
    [devices, selectedId],
  );

  useEffect(() => {
    try {
      window.localStorage.setItem(HISTORY_KEY, JSON.stringify(history));
    } catch {
      // Local persistence is optional; the session still works without it.
    }
  }, [history]);

  const pushLog = useCallback((entry: LogEntry) => {
    setLogs((current) => [entry, ...current].slice(0, 200));
  }, []);

  /** Read the counters of one device and fold the result into the state. */
  const readDevice = useCallback(
    async (device: PrinterDevice, announce = true) => {
      const next = await transport.refresh(device);
      setCounters(next);
      setDevices((current) =>
        current.map((item) =>
          item.id === device.id ? { ...item, counters: next, connected: true } : item,
        ),
      );
      if (announce) {
        pushLog(
          createLog(
            'system',
            tRef.current('logmsg.countersRead'),
            `${device.model} · ${tRef.current('dash.mainPad')} ${next.mainPad}% · ${tRef.current('dash.borderlessPad')} ${next.borderlessPad}%`,
          ),
        );
      }
      return next;
    },
    [pushLog, transport],
  );

  const scan = useCallback(async () => {
    setIsScanning(true);
    setConnection('scanning');
    try {
      const found = await transport.scan();
      setDevices(found);
      const current = found.find((device) => device.id === selectedId) ?? found[0];
      if (!current) {
        setSelectedId(null);
        setCounters(emptyCounters());
        setConnection('offline');
        pushLog(createLog('connection', tRef.current('logmsg.noPrinter')));
        return;
      }
      setSelectedId(current.id);
      setConnection('online');
      pushLog(
        createLog(
          'connection',
          tRef.current('logmsg.detected'),
          `${current.model} · ${current.serial} · ${current.port}`,
        ),
      );
      try {
        await readDevice(current);
      } catch (error) {
        pushLog(
          createLog('warning', tRef.current('logmsg.readFailed'), messageOf(error)),
        );
      }
    } catch (error) {
      setDevices([]);
      setSelectedId(null);
      setCounters(emptyCounters());
      setConnection('offline');
      pushLog(createLog('warning', tRef.current('logmsg.scanFailed'), messageOf(error)));
    } finally {
      setIsScanning(false);
    }
  }, [pushLog, readDevice, selectedId, transport]);

  // Detect the printer as soon as the bench opens.
  const autoScanned = useRef(false);
  useEffect(() => {
    if (autoScanned.current) return;
    autoScanned.current = true;
    void scan();
  }, [scan]);

  const selectDevice = useCallback(
    (id: string) => {
      const device = devices.find((item) => item.id === id);
      if (!device) return;
      setSelectedId(id);
      setCounters(device.counters);
      setConnection(device.connected ? 'online' : 'offline');
      setResetProgress({
        stage: 'idle',
        label: tRef.current('progress.ready'),
        percent: 0,
      });
      pushLog(
        createLog(
          'connection',
          tRef.current('logmsg.selectionChanged'),
          `${device.model} · ${device.port}`,
        ),
      );
      if (device.connected) {
        void readDevice(device, false).catch((error) => {
          pushLog(createLog('warning', tRef.current('logmsg.readFailed'), messageOf(error)));
        });
      }
    },
    [devices, pushLog, readDevice],
  );

  const refresh = useCallback(async () => {
    if (!selected || connection === 'offline' || connection === 'scanning') return;
    setIsRefreshing(true);
    try {
      await readDevice(selected);
    } catch (error) {
      pushLog(createLog('warning', tRef.current('logmsg.readFailed'), messageOf(error)));
    } finally {
      setIsRefreshing(false);
    }
  }, [connection, pushLog, readDevice, selected]);

  const toggleConnection = useCallback(async () => {
    if (!selected) return;
    if (connection === 'online') {
      setConnection('offline');
      setDevices((current) =>
        current.map((device) =>
          device.id === selected.id ? { ...device, connected: false } : device,
        ),
      );
      pushLog(
        createLog('connection', tRef.current('logmsg.released'), selected.model),
      );
      return;
    }

    setConnection('scanning');
    try {
      await readDevice(selected);
      setConnection('online');
      pushLog(
        createLog(
          'connection',
          tRef.current('logmsg.reconnected'),
          `${selected.model} · ${selected.port}`,
        ),
      );
    } catch (error) {
      setConnection('offline');
      pushLog(createLog('warning', tRef.current('logmsg.reconnectFailed'), messageOf(error)));
    }
  }, [connection, pushLog, readDevice, selected]);

  const runReset = useCallback(
    async (confirm: string) => {
      if (!selected) throw new Error(tRef.current('err.noPrinter'));
      setResetProgress({
        stage: 'idle',
        label: tRef.current('progress.ready'),
        percent: 0,
      });
      try {
        const before = counters.mainPad;
        const next = await transport.reset({
          device: selected,
          confirm,
          onProgress: setResetProgress,
        });
        setCounters(next);
        setDevices((current) =>
          current.map((device) =>
            device.id === selected.id
              ? {
                  ...device,
                  counters: next,
                  lastReset: new Date().toISOString().slice(0, 10),
                }
              : device,
          ),
        );
        const record: ResetRecord = {
          id: globalThis.crypto?.randomUUID?.() ?? `${Date.now()}`,
          at: new Date().toISOString(),
          model: selected.model,
          serial: selected.serial,
          before,
          after: next.mainPad,
        };
        setHistory((current) => [record, ...current].slice(0, 100));
        pushLog(
          createLog(
            'reset',
            tRef.current('logmsg.resetDone'),
            `${selected.model} · ${before}% → ${next.mainPad}%`,
          ),
        );
        return next;
      } catch (error) {
        const message = messageOf(error);
        setResetProgress({ stage: 'error', label: message, percent: 0 });
        pushLog(createLog('warning', tRef.current('logmsg.resetFailed'), message));
        throw error;
      }
    },
    [counters.mainPad, pushLog, selected, transport],
  );

  /**
   * Prove the EEPROM write path without touching a counter: the backend writes
   * one byte of the reset table back with its own value and re-reads it.
   */
  const selfTest = useCallback(async (): Promise<SelfTestResult> => {
    if (!selected) throw new Error(tRef.current('err.noDevice'));
    try {
      const result = await transport.selfTest(selected);
      pushLog(
        createLog(
          result.unchanged ? 'system' : 'warning',
          tRef.current(result.unchanged ? 'logmsg.selfTestOk' : 'logmsg.selfTestBad'),
          `${selected.model} · ${result.address} · ${result.before} → ${result.after}`,
        ),
      );
      return result;
    } catch (error) {
      const message = messageOf(error);
      pushLog(createLog('warning', tRef.current('logmsg.selfTestBad'), message));
      throw error;
    }
  }, [pushLog, selected, transport]);

  const clearResetProgress = useCallback(
    () =>
      setResetProgress({
        stage: 'idle',
        label: tRef.current('progress.ready'),
        percent: 0,
      }),
    [],
  );

  const clearHistory = useCallback(() => {
    setHistory([]);
    pushLog(createLog('system', tRef.current('logmsg.historyCleared')));
  }, [pushLog]);

  return {
    devices,
    selected,
    counters,
    connection,
    logs,
    history,
    resetProgress,
    isRefreshing,
    isScanning,
    isNative: bridgeIsNative(),
    selectDevice,
    scan,
    refresh,
    toggleConnection,
    runReset,
    selfTest,
    clearResetProgress,
    clearHistory,
  };
}

export type PrinterSession = ReturnType<typeof usePrinterSession>;
