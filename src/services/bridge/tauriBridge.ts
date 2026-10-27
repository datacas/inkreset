import { invoke, isTauri } from '@tauri-apps/api/core';

export interface UsbPrinterInfo {
  path: string;
  instanceId: string;
  friendlyName: string;
  hardwareId: string;
  vendorId: string;
  productId: string;
  ieee1284Id: string | null;
  manufacturer: string | null;
  model: string | null;
  serial: string | null;
}

export interface BridgeStatus {
  platform: string;
  available: boolean;
  discovery: boolean;
  counterProtocol: boolean;
  /** Number of printer models in the embedded device database. */
  profiles?: number;
  detail: string;
}

export const bridgeIsNative = (): boolean => {
  try {
    return isTauri();
  } catch {
    return false;
  }
};

export const readBridgeStatus = (): Promise<BridgeStatus> =>
  invoke<BridgeStatus>('bridge_status');

export const listUsbPrinters = (): Promise<UsbPrinterInfo[]> =>
  invoke<UsbPrinterInfo[]>('list_usb_printers');

export interface ProfileSummary {
  key: string;
  title: string;
  short: string;
  model: string;
  chain: string[];
  brand: string;
  class: string;
  hasService: boolean;
  counters: number;
  resetPairs: number;
}

export interface CounterValue {
  index: number;
  registers: number[];
  value: number;
  max: number;
  percent: number;
}

export interface CounterReport {
  model: string;
  chain: string[];
  socket: number;
  sendSize: number;
  recvSize: number;
  factory: number[];
  counters: CounterValue[];
  note: string;
}

export interface ResetReport {
  model: string;
  chain: string[];
  bytesWritten: number;
  counters: CounterValue[];
  note: string;
}

export interface SelfTestReport {
  model: string;
  /** EEPROM address exercised by the test. */
  address: number;
  before: number;
  after: number;
  /** True when the byte read back equals the byte that was written. */
  unchanged: boolean;
  note: string;
}

/** Every printer model in the embedded device database. */
export const listDeviceProfiles = (query?: string): Promise<ProfileSummary[]> =>
  invoke<ProfileSummary[]>('list_device_profiles', { query: query ?? null });

/**
 * Enter IEEE 1284.4, read every waste counter of the matched profile and leave
 * 1284.4 mode again. `model` must be the `MDL:` value reported by the printer.
 */
export const readWasteCounters = (
  devicePath: string,
  model: string,
): Promise<CounterReport> =>
  invoke<CounterReport>('read_waste_counters', { devicePath, model });

/**
 * Apply the `<reset>` table. `confirm` must equal the printer model
 * exactly; the backend refuses anything else.
 */
export const resetWasteCounters = (
  devicePath: string,
  model: string,
  confirm: string,
): Promise<ResetReport> =>
  invoke<ResetReport>('reset_waste_counters', { devicePath, model, confirm });

/**
 * Write one byte of the profile's own reset table back unchanged.
 *
 * This exercises the full EEPROM write path (1284.4 session, write frame and
 * the model write key) without touching any counter, so the reset plumbing can
 * be proven on a real printer before a destructive write.
 */
export const selfTestWrite = (
  devicePath: string,
  model: string,
): Promise<SelfTestReport> =>
  invoke<SelfTestReport>('self_test_write', { devicePath, model });
