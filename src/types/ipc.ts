// Typed IPC contract mirrored by src-tauri/src/events.rs and command modules.
// Keep both sides in sync when changing anything here.

export type Source = "system" | "mic";

export type PipelineState =
  | "idle"
  | "starting"
  | "listening"
  | "speech"
  | "paused"
  | "reconnecting"
  | "error";

export type OverlayMode = "full" | "interview";

export interface Device {
  id: string;
  name: string;
  isDefault: boolean;
}

export interface DeviceList {
  inputs: Device[];
  outputs: Device[];
}

export interface StartPipelineParams {
  source: Source;
  deviceId: string | null;
  /** "auto" or ISO 639-1 code */
  sourceLang: string;
  /** ISO 639-1 code */
  targetLang: string;
  sttModel: string;
  translationModel: string;
  useServerVad: boolean;
  translationStyle: "natural" | "literal";
}

export interface TranscriptPayload {
  source: Source;
  segmentId: string;
  text: string;
  ts: number;
}

export interface TranslationDeltaPayload {
  source: Source;
  segmentId: string;
  delta: string;
}

export interface TranslationFinalPayload {
  source: Source;
  segmentId: string;
  text: string;
  targetLang: string;
}

export interface PipelineStatusPayload {
  source: Source;
  state: PipelineState;
  detail?: string;
}

export interface AudioLevelPayload {
  source: Source;
  rms: number;
}

export type AppErrorCode =
  | "no_device"
  | "invalid_key"
  | "network"
  | "rate_limit"
  | "device_lost"
  | "session_limit"
  | "keyring"
  | "translation_timeout"
  | "internal";

export interface AppErrorPayload {
  code: AppErrorCode;
  message: string;
  source?: Source;
  recoverable: boolean;
}

export type ShortcutAction =
  | "toggle_overlay"
  | "start_stop"
  | "swap_direction"
  | "pause_resume"
  | "clear_history";

export interface ShortcutActionPayload {
  action: ShortcutAction;
}

export interface TestApiKeyResult {
  ok: boolean;
  error?: string;
}
