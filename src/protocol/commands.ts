/**
 * Per-feature command encoders/decoders, reconstructed from the minified bundle.
 *
 * Each function maps a high-level config object to/from the on-wire byte layout
 * and drives it through the chunked `transfer` transport in `core.ts`.
 * Payload layouts are verbatim from the upstream bundle — see the minified
 * symbol noted on each function (and the symbol map in ../../README.md).
 *
 * NOTE: upstream gates many timeouts on `deviceInfo.frameVersion` (2000 ms when
 * frameVersion === 1, else 500 ms). Pass it via `frameVersion` where relevant.
 */
import { CMD, FactoryResetType, REPORT_ID, transfer, concatPayload, buildPacket, DEFAULT_REPORT_SIZE } from "./core";

const timeoutFor = (frameVersion?: number) => (frameVersion === 1 ? 2000 : 500);

// ─── Game mode / global settings ── minified getGameMode `Re` / setGameMode `qe`
// 56-byte payload.

export interface GameMode {
  gameMode: number;
  fnSwitch: number;
  sleepTime: number;
  keyDelay: number;
  reportRate: number;
  systemMode: number;
  tftDisplayTime: number;
  /** Stored on the wire as percent*100; exposed as a 0..1 fraction. */
  topDeadZone: number;
  bottomDeadZone: number;
  stabilityMode: number;
  autoCalibration: number;
  singleKeyWakeup: number;
}

export async function getGameMode(device: HIDDevice, frameVersion?: number): Promise<GameMode> {
  const o = concatPayload(
    await transfer(device, { cmd: CMD.GET_GAME_MODE, contentSize: 56, timeout: timeoutFor(frameVersion) }),
    56,
  );
  return {
    gameMode: o[1],
    fnSwitch: o[2],
    sleepTime: o[3],
    keyDelay: o[4],
    reportRate: o[5],
    systemMode: o[6],
    tftDisplayTime: o[7],
    topDeadZone: o[8] / 100,
    bottomDeadZone: o[9] / 100,
    stabilityMode: o[11],
    autoCalibration: o[14],
    singleKeyWakeup: o[15],
  };
}

export async function setGameMode(device: HIDDevice, v: GameMode, frameVersion?: number): Promise<boolean> {
  const e = new Uint8Array(56).fill(0);
  e[1] = v.gameMode || 0;
  e[2] = v.fnSwitch || 0;
  e[3] = v.sleepTime || 0;
  e[4] = v.keyDelay || 0;
  e[5] = v.reportRate || 0;
  e[6] = v.systemMode || 0;
  e[7] = v.tftDisplayTime || 0;
  e[8] = (v.topDeadZone * 100) || 0;
  e[9] = (v.bottomDeadZone * 100) || 0;
  e[11] = v.stabilityMode || 0;
  e[14] = v.autoCalibration || 0;
  e[15] = v.singleKeyWakeup || 0;
  await transfer(device, { cmd: CMD.SET_GAME_MODE, contentSize: 56, data: e, timeout: timeoutFor(frameVersion) });
  return true;
}

// ─── Keymap ── minified getKeyData `Ne`
// 512-byte payload = 128 keys × 4 bytes. Upstream then maps raw entries to
// labels via a per-model layout (omitted here — we expose the raw entries).

export interface RawKeyEntry {
  pageType: number;
  param1: number;
  param2: number;
  param3: number;
}

export async function getKeyData(device: HIDDevice, frameVersion?: number): Promise<RawKeyEntry[]> {
  const e = concatPayload(
    await transfer(device, { cmd: CMD.GET_KEY, contentSize: 512, timeout: timeoutFor(frameVersion) }),
    512,
  );
  const keys: RawKeyEntry[] = [];
  for (let i = 0; i < 128; i++) {
    const b = i * 4;
    keys.push({ pageType: e[b], param1: e[b + 1], param2: e[b + 2], param3: e[b + 3] });
  }
  return keys;
}

// ─── Lighting effect ── minified getLEDEffect `Ue` / setLEDEffect `mt`
// 16-byte payload.

export interface LedEffect {
  mode: number;
  red: number;
  green: number;
  blue: number;
  /** byte 4: always written as 255 on set (driverSetting). */
  driverSetting: number;
  secondaryRed: number;
  secondaryGreen: number;
  secondaryBlue: number;
  colorMode: number;
  brightness: number;
  speed: number;
  direction: number;
  effectModeType: number;
  checkCodeL: number;
  checkCodeH: number;
}

export async function getLedEffect(device: HIDDevice, frameVersion?: number): Promise<LedEffect> {
  const o = concatPayload(
    await transfer(device, { cmd: CMD.GET_LED_EFFECT, contentSize: 16, timeout: timeoutFor(frameVersion) }),
    16,
  );
  return {
    mode: o[0], red: o[1], green: o[2], blue: o[3], driverSetting: o[4],
    secondaryRed: o[5], secondaryGreen: o[6], secondaryBlue: o[7],
    colorMode: o[8], brightness: o[9], speed: o[10], direction: o[11],
    effectModeType: o[12], checkCodeL: o[14], checkCodeH: o[15],
  };
}

export async function setLedEffect(device: HIDDevice, v: LedEffect): Promise<boolean> {
  const e = new Uint8Array(16).fill(0);
  e[0] = v.mode; e[1] = v.red; e[2] = v.green; e[3] = v.blue;
  e[4] = 255; // driverSetting forced to 255 by upstream
  e[5] = v.secondaryRed; e[6] = v.secondaryGreen; e[7] = v.secondaryBlue;
  e[8] = v.colorMode; e[9] = v.brightness; e[10] = v.speed; e[11] = v.direction;
  e[12] = v.effectModeType;
  await transfer(device, { cmd: CMD.SET_LED_EFFECT, contentSize: 16, data: e });
  return true;
}

// ─── Rapid Trigger (magnetic axis) ── minified getMagneticAxisRT `Fe`
// 1024-byte payload = 128 keys × 8 bytes. Stroke/RT scaling depends on
// deviceInfo.rtPrecision (0 → /100; 1 → strokes /100, RT /1000; 2 → both /1000).

export interface MagneticAxisRT {
  axisType: number;
  isWholeFast: boolean;
  isRampageMode: boolean;
  triggerKeyStroke: number;
  pressRT: number;
  releaseRT: number;
}

export async function getMagneticAxisRT(
  device: HIDDevice,
  rtPrecision = 0,
  frameVersion?: number,
): Promise<MagneticAxisRT[]> {
  const o = concatPayload(
    await transfer(device, { cmd: CMD.GET_MAGNETIC_AXIS_RT, contentSize: 1024, timeout: timeoutFor(frameVersion) }),
    1024,
  );
  const rtScale = rtPrecision > 0 ? 1000 : 100;
  const strokeScale = rtPrecision === 2 ? 1000 : 100;
  const out: MagneticAxisRT[] = [];
  for (let i = 0; i < 128; i++) {
    const b = i * 8;
    const flags = o[b + 1];
    out.push({
      axisType: o[b],
      isWholeFast: (flags & 1) !== 0,
      isRampageMode: (flags & 2) !== 0,
      triggerKeyStroke: (o[b + 2] | (o[b + 3] << 8)) / strokeScale,
      pressRT: (o[b + 4] | (o[b + 5] << 8)) / rtScale,
      releaseRT: (o[b + 6] | (o[b + 7] << 8)) / rtScale,
    });
  }
  return out;
}

// ─── Factory reset ── minified factoryReset `Ae`
// Single 0xAA packet, no chunking; reset type goes in the `len` byte.

export async function factoryReset(
  device: HIDDevice,
  type: FactoryResetType = FactoryResetType.RESET_ALL,
): Promise<boolean> {
  const reportSize =
    device?.collections?.[0]?.outputReports?.[0]?.items?.[0]?.reportCount || DEFAULT_REPORT_SIZE;
  const packet = buildPacket(CMD.SET_FACTORY_RESET, type, 0, undefined, reportSize);
  await device.sendReport(REPORT_ID, packet);
  await new Promise((r) => setTimeout(r, 100));
  return true;
}

export const clearCalibration = (device: HIDDevice) =>
  factoryReset(device, FactoryResetType.CLEAR_CALIBRATION);
export const resetAll = (device: HIDDevice) => factoryReset(device, FactoryResetType.RESET_ALL);

// ─── Calibration data notify ── minified `Oe` (parser for GET_MAGNETIC_AXIS_CALIBRATION_DATA)

export interface CalibrationSample {
  keyValue: number;
  calibrationStatus: number;
  maxValue: number;
  minValue: number;
  currentValue: number;
  keyStroke: number;
  maxStroke: number;
}

/** Parse a raw inputreport buffer into a calibration sample, or null if it isn't one. */
export function parseCalibrationSample(buf: Uint8Array): CalibrationSample | null {
  if (!(buf.length >= 13 && buf[0] === 0x55 && buf[1] === CMD.GET_MAGNETIC_AXIS_CALIBRATION_DATA)) {
    return null;
  }
  return {
    keyValue: buf[2],
    calibrationStatus: buf[3],
    maxValue: buf[4] | (buf[5] << 8),
    minValue: (buf[6] | (buf[7] << 8)) & 32767,
    currentValue: buf[8] | (buf[9] << 8),
    keyStroke: buf[10] | (buf[11] << 8),
    maxStroke: buf[12] | (buf[13] << 8),
  };
}
