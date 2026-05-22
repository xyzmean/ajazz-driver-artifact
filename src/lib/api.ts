/**
 * Frontend ↔ Rust bridge. All HID communication happens in Rust (hidapi),
 * because Tauri webviews do not expose WebHID — the UI only calls invoke().
 */
import { invoke } from "@tauri-apps/api/core";

export interface DeviceSummary {
  path: string;
  vendorId: number;
  productId: number;
  product: string | null;
  manufacturer: string | null;
  /** Resolved keyboard model name from our table, if known. */
  modelName: string | null;
}

export interface DeviceInfo {
  romSize: number;
  macroSpaceSize: number;
  vendorId: number;
  productId: number;
  version: number;
  sensor: number;
  workMode: number;
  batteryLevel: number;
  chargeStatus: number;
  currentProfile: number;
  rtPrecision: number;
  frameVersion: number;
  lightingVersion: number;
}

/** List connected Ajazz keyboards visible to the OS HID layer. */
export const listDevices = () => invoke<DeviceSummary[]>("list_devices");

/** Read device info from a keyboard by its HID path, via our Rust protocol port. */
export const getDeviceInfo = (path: string) => invoke<DeviceInfo>("get_device_info", { path });

export interface GameMode {
  gameMode: number;
  fnSwitch: number;
  sleepTime: number;
  keyDelay: number;
  reportRate: number;
  systemMode: number;
  tftDisplayTime: number;
  topDeadZone: number;
  bottomDeadZone: number;
  stabilityMode: number;
  autoCalibration: number;
  singleKeyWakeup: number;
}

export interface LedEffect {
  mode: number;
  red: number;
  green: number;
  blue: number;
  driverSetting: number;
  secondaryRed: number;
  secondaryGreen: number;
  secondaryBlue: number;
  colorMode: number;
  brightness: number;
  speed: number;
  direction: number;
  effectModeType: number;
}

export interface MagneticAxisRT {
  axisType: number;
  isWholeFast: boolean;
  isRampageMode: boolean;
  triggerKeyStroke: number;
  pressRt: number;
  releaseRt: number;
}

export const openSettingsWindow = (path: string, model: string) =>
  invoke<void>("open_settings_window", { path, model });

export const getGameMode = (path: string, frameVersion: number) =>
  invoke<GameMode>("get_game_mode", { path, frameVersion });

export const setGameMode = (path: string, config: GameMode, frameVersion: number) =>
  invoke<void>("set_game_mode", { path, config, frameVersion });

export const getLedEffect = (path: string, frameVersion: number) =>
  invoke<LedEffect>("get_led_effect", { path, frameVersion });

export const setLedEffect = (path: string, effect: LedEffect) =>
  invoke<void>("set_led_effect", { path, effect });

export const getMagneticRt = (path: string, rtPrecision: number, frameVersion: number) =>
  invoke<MagneticAxisRT[]>("get_magnetic_rt", { path, rtPrecision, frameVersion });

export const setMagneticRt = (
  path: string,
  rtPrecision: number,
  data: MagneticAxisRT[],
  frameVersion: number
) => invoke<void>("set_magnetic_rt", { path, rtPrecision, data, frameVersion });

export const factoryReset = (path: string, resetType: number) =>
  invoke<void>("factory_reset", { path, resetType });
