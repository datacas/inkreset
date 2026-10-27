import type {
  PrinterDevice,
  PrinterTransport,
  ResetRequest,
  SelfTestResult,
  WasteCounters,
} from '../../types/printer';
import type { Translate } from '../../i18n';
import {
  bridgeIsNative,
  listUsbPrinters,
  readWasteCounters,
  resetWasteCounters,
  selfTestWrite,
  type CounterValue,
  type UsbPrinterInfo,
} from './tauriBridge';

const emptyCounters = (): WasteCounters => ({
  mainPad: 0,
  borderlessPad: 0,
  inkAbsorber: 0,
  readAt: '',
});

const percentOf = (counters: CounterValue[], index: number): number => {
  const value = counters[index]?.percent ?? 0;
  return Math.round(Math.min(100, Math.max(0, value)));
};

/**
 * Map the backend counter report onto the three gauges.
 *
 * The device profiles list the main pad first and the platen / borderless
 * pad second, which is the order of the `<counter>` blocks in the embedded
 * device database. A profile with a third counter is the ink absorber.
 */
const toWasteCounters = (counters: CounterValue[]): WasteCounters => ({
  mainPad: percentOf(counters, 0),
  borderlessPad: percentOf(counters, 1),
  inkAbsorber: percentOf(counters, 2),
  readAt: new Date().toISOString(),
});

const seriesOf = (model: string): string => {
  const text = model.toUpperCase();
  if (/ECOTANK|^E?T-|(^|\s)L\d/.test(text)) return 'EcoTank';
  if (/WORKFORCE|WF-/.test(text)) return 'WorkForce';
  if (/EXPRESSION|XP-/.test(text)) return 'Expression';
  if (/STYLUS/.test(text)) return 'Stylus';
  return 'Epson';
};

const toDevice = (printer: UsbPrinterInfo): PrinterDevice => {
  const model = printer.model ?? printer.friendlyName ?? 'USB printer';
  const port = [printer.vendorId, printer.productId].filter(Boolean).join(':');
  return {
    id: printer.path,
    model,
    series: seriesOf(model),
    serial: printer.serial ?? '—',
    firmware: '—',
    port: port ? `USB ${port}` : 'USB',
    transport: 'USB 2.0',
    driver: printer.manufacturer ?? model,
    connected: true,
    counters: emptyCounters(),
    lastReset: null,
  };
};

/**
 * Real USB transport. Every call goes through the Tauri backend, which owns
 * SetupAPI discovery and the IEEE 1284.4 session. There is no offline or demo
 * fallback: without the native bridge the bench stays empty.
 */
export class UsbPrinterTransport implements PrinterTransport {
  constructor(private readonly t: Translate) {}

  async scan(): Promise<PrinterDevice[]> {
    if (!bridgeIsNative()) {
      throw new Error(this.t('err.bridgeUnavailable'));
    }
    const printers = await listUsbPrinters();
    return printers.map(toDevice);
  }

  async refresh(device: PrinterDevice): Promise<WasteCounters> {
    const report = await readWasteCounters(device.id, device.model);
    return toWasteCounters(report.counters);
  }

  async reset({ device, confirm, onProgress }: ResetRequest): Promise<WasteCounters> {
    onProgress({
      stage: 'handshake',
      label: this.t('progress.handshake'),
      percent: 15,
    });
    onProgress({
      stage: 'writing',
      label: this.t('progress.writing'),
      percent: 45,
    });
    const report = await resetWasteCounters(device.id, device.model, confirm);
    onProgress({
      stage: 'verify',
      label: this.t('progress.verify'),
      percent: 85,
    });
    const next = toWasteCounters(report.counters);
    onProgress({
      stage: 'complete',
      label: this.t('progress.complete'),
      percent: 100,
    });
    return next;
  }

  async selfTest(device: PrinterDevice): Promise<SelfTestResult> {
    const report = await selfTestWrite(device.id, device.model);
    return {
      model: report.model,
      address: report.address,
      before: report.before,
      after: report.after,
      unchanged: report.unchanged,
      note: report.note,
    };
  }
}
