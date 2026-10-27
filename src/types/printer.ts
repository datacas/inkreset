export type ConnectionState = 'online' | 'offline' | 'scanning';

export interface WasteCounters {
  mainPad: number;
  borderlessPad: number;
  inkAbsorber: number;
  readAt: string;
}

export interface PrinterDevice {
  id: string;
  model: string;
  series: string;
  serial: string;
  firmware: string;
  port: string;
  transport: 'USB 2.0';
  driver: string;
  connected: boolean;
  counters: WasteCounters;
  lastReset: string | null;
}

export type LogKind = 'reset' | 'connection' | 'system' | 'warning';

export interface LogEntry {
  id: string;
  at: string;
  kind: LogKind;
  message: string;
  detail?: string;
}

export interface ResetRecord {
  id: string;
  at: string;
  model: string;
  serial: string;
  before: number;
  after: number;
}

export type ResetStage =
  | 'idle'
  | 'handshake'
  | 'writing'
  | 'verify'
  | 'complete'
  | 'error';

export interface ResetProgress {
  stage: ResetStage;
  label: string;
  percent: number;
}

export interface ResetRequest {
  device: PrinterDevice;
  /** Must equal the printer model exactly; the backend refuses anything else. */
  confirm: string;
  onProgress: (progress: ResetProgress) => void;
}

export interface SelfTestResult {
  model: string;
  address: number;
  before: number;
  after: number;
  unchanged: boolean;
  note: string;
}

export interface PrinterTransport {
  scan(): Promise<PrinterDevice[]>;
  refresh(device: PrinterDevice): Promise<WasteCounters>;
  reset(request: ResetRequest): Promise<WasteCounters>;
  /** Write one byte back unchanged to prove the write path is accepted. */
  selfTest(device: PrinterDevice): Promise<SelfTestResult>;
}

export interface ModelRow {
  series: string;
  models: string[];
  note?: string;
}
