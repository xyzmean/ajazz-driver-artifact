/**
 * Ajazz keyboard WebHID protocol — reverse-engineered core.
 *
 * Extracted by hand from the minified production bundle
 * (`app/assets/index-BosuIawc.js`, deminified into `index.core.pretty.js`).
 * The upstream bundle ships no sourcemaps (the CDN returns the SPA index.html
 * for any `*.js.map` request), so this module is reconstructed from the
 * minified source. Original (minified) symbol names are noted in comments so
 * findings can be re-derived after an upstream rebuild.
 *
 * This file owns the protocol independently of the upstream UI bundle: it is
 * the foundation a custom frontend / CLI / cross-platform port builds on.
 *
 * ── Wire format ──────────────────────────────────────────────────────────
 *   Transport: WebHID. Reports are sent with report ID 0 (`REPORT_ID`).
 *   Report length is taken from the device's output report descriptor
 *   (`collections[0].outputReports[0].items[0].reportCount`), default 32 bytes.
 *
 *   Request packet (built by `buildPacket`, minified `P`):
 *     [0]    0xAA            request magic (REQUEST_HEADER)
 *     [1]    cmd             opcode (see CMD)
 *     [2]    len             payload length carried by THIS packet
 *     [3]    addr & 0xFF     target address, low byte (little-endian)
 *     [4]    addr >> 8       target address, high byte
 *     [5..7] header bytes    up to 3 optional "otherHeader" bytes; when absent,
 *                            [6] doubles as the last-packet flag (1 = last)
 *     [8..]  payload         chunk data
 *
 *   Response packet (parsed by `parseResponse`, minified `Pe`):
 *     [0]    0x55            response magic (RESPONSE_HEADER); else error
 *     [1]    cmd             echoes the request opcode
 *     [2]    lenOrType
 *     [3..4] addr            little-endian
 *     [8..]  data            payload
 *
 *   NOTE: the generic transport carries NO checksum. A few specific commands
 *   (e.g. SET_MUSIC_DATA) append their own checksum at the last byte
 *   (`buf[31] = sum(buf[0..30]) & 0xFF`); that lives in those commands, not here.
 */

// ─── Constants ──────────────────────────────────────────────────────────────

/** WebHID report ID used for every transfer (minified `V`). */
export const REPORT_ID = 0;

/** First byte of every request packet (minified literal `170`). */
export const REQUEST_HEADER = 0xaa;

/** First byte of every valid response packet (minified literal `85`). */
export const RESPONSE_HEADER = 0x55;

/** Bytes [0..7] are header; payload starts at byte 8. */
export const HEADER_SIZE = 8;

/** Fallback report length when the device descriptor doesn't specify one. */
export const DEFAULT_REPORT_SIZE = 32;

/** Default usagePage probed when opening a device (minified `z`, 0xFF67). */
export const DEFAULT_USAGE_PAGE = 65383;

// ─── Command opcodes ──────────────────────────────────────────────────────────
// Minified enum `E`, exported upstream as `CMD`. Verbatim names/values.

export enum CMD {
  COMMUNICATION_START = 1,
  COMMUNICATION_END = 2,
  SET_FACTORY_RESET = 15,
  GET_DEVICE_INFO = 16,
  GET_GAME_MODE = 17,
  GET_KEY = 18,
  GET_LED_EFFECT = 19,
  GET_CUSTOM_LED_DATA = 20,
  GET_MACRO = 21,
  GET_FN_KEY = 22,
  GET_MAGNETIC_AXIS_RT = 23,
  GET_MAGNETIC_AXIS_DKS_DATA = 24,
  GET_LIGHT_BOX = 27,
  GET_DEFAULT_FN_KEY_MATRIX = 28,
  GET_DEFAULT_KEY_MATRIX = 31,
  SET_GAME_MODE = 33,
  SET_KEY = 34,
  SET_LED_EFFECT = 35,
  SET_CUSTOM_LED_DATA = 36,
  SET_MACRO = 37,
  SET_FN_KEY = 38,
  SET_MAGNETIC_AXIS_RT = 39,
  SET_MAGNETIC_AXIS_DKS_DATA = 40,
  SET_DOT_MATRIX_MODE = 42,
  SET_LIGHT_BOX = 43,
  SET_KEYBOARD_CUSTOM_FUNCTION_ON = 48,
  SET_KEYBOARD_CUSTOM_FUNCTION_OFF = 49,
  GET_LED_DATA = 50,
  GET_ALL_LIGHTS_RGB = 51,
  SET_TEMPORARY_COMMAND_DATA = 52,
  SET_MUSIC_DATA = 53,
  CLEAR_LED_DATA = 54,
  GET_ALL_LIGHTS_RGB_24G = 55,
  GET_ALL_LIGHTS_RGB_24G_64_BYTE = 59,
  SET_MUSIC_DATA_24G_64_BYTE = 60,
  SET_LED_BOOT_ANIMATION = 64,
  SET_LED_USER_ANIMATION = 65,
  SET_LED_DATA = 66,
  SET_FLASH_DOWNLOAD = 79,
  SET_TFT_USER_ANIMATION = 80,
  SET_TFT_BUILT_IN_INDEX = 81,
  GET_MAGNETIC_AXIS_KEY_STATUS = 96,
  SET_CALIBRATION_ON = 100,
  SET_CALIBRATION_OFF = 101,
  SET_SIMULATION_TEST_ON = 102,
  SET_SIMULATION_TEST_OFF = 103,
  GET_MAGNETIC_AXIS_STATUS = 104,
  SET_CALIBRATION_ON_V2 = 105,
  SET_CALIBRATION_OFF_V2 = 106,
  GET_DEVICE_NOTIFY = 250,
  GET_MAGNETIC_AXIS_CALIBRATION_DATA = 251,
  GET_24G_DISCONNECT_NOTIFY = 252,
}

/** Factory-reset selector bitmask (minified `pe`, exported as `FACTORY_RESET_TYPE`). */
export enum FactoryResetType {
  KEY_RESET = 1,
  LIGHTING_RESET = 2,
  MACRO_RESET = 4,
  CLEAR_CALIBRATION = 5,
  RESET_ALL = 255,
}

// ─── Packet framing ───────────────────────────────────────────────────────────

export interface PacketOptions {
  /** Optional 1–3 extra header bytes placed at [5..7] (minified `o`). */
  otherHeader?: number[];
  /** Fully custom header written from byte 0 (minified `e`); overrides all framing. */
  customHeader?: Uint8Array;
  /** Marks this as the final packet of a multi-packet write (sets [6] when no otherHeader). */
  isLastPacket?: boolean;
}

/**
 * Build a single request packet. Minified `P`.
 * @param cmd      opcode
 * @param len      payload length carried by this packet (byte [2])
 * @param addr     little-endian target address (bytes [3..4])
 * @param payload  chunk data written from byte 8
 * @param size     total report size (default 32)
 */
export function buildPacket(
  cmd: CMD,
  len: number,
  addr: number,
  payload?: Uint8Array,
  size = DEFAULT_REPORT_SIZE,
  opts: PacketOptions = {},
): Uint8Array<ArrayBuffer> {
  // Backed by a plain ArrayBuffer so it satisfies WebHID's BufferSource (TS 5.7+).
  const buf = new Uint8Array(new ArrayBuffer(size));

  // customHeader path: caller controls the header entirely.
  if (opts.customHeader) {
    buf.set(opts.customHeader, 0);
    if (payload) buf.set(payload, opts.customHeader.length);
    return buf;
  }

  buf[0] = REQUEST_HEADER;
  buf[1] = cmd;
  buf[2] = len;
  buf[3] = addr & 0xff;
  buf[4] = (addr >> 8) & 0xff;

  if (opts.otherHeader && Array.isArray(opts.otherHeader)) {
    for (let i = 0; i < Math.min(opts.otherHeader.length, 3); i++) {
      if (opts.otherHeader[i] !== undefined) buf[5 + i] = opts.otherHeader[i] & 0xff;
    }
  }

  // When no meaningful otherHeader[1], byte [6] is the last-packet flag.
  const hasHeaderByte1 = opts.otherHeader && opts.otherHeader.length >= 2 && opts.otherHeader[1] !== undefined;
  if (!hasHeaderByte1) buf[6] = opts.isLastPacket ? 1 : 0;

  if (payload) buf.set(payload, HEADER_SIZE);
  return buf;
}

export interface ParsedResponse {
  header: number;
  cmd: number;
  lenOrType: number;
  addr: number;
  /** Payload, bytes [8..]. */
  data: Uint8Array;
}

/** Parse a response packet. Returns null on a bad header. Minified `Pe`. */
export function parseResponse(buf: Uint8Array): ParsedResponse | null {
  if (buf[0] !== RESPONSE_HEADER) {
    console.error(`Bad response header: 0x${buf[0].toString(16)}`); // 数据包头错误
    return null;
  }
  return {
    header: buf[0],
    cmd: buf[1],
    lenOrType: buf[2],
    addr: buf[3] | (buf[4] << 8),
    data: buf.slice(HEADER_SIZE),
  };
}

// ─── Request / response ───────────────────────────────────────────────────────

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * Resolve once an inputreport whose cmd (and optional addr) matches arrives,
 * or reject after `timeout` ms. Minified `j`.
 */
export function waitForResponse<T = Uint8Array>(
  device: HIDDevice,
  match: { command: CMD; timeout?: number; addr?: number },
  transform: (raw: Uint8Array) => T = (raw) => raw as unknown as T,
): Promise<T> {
  const { command, timeout = 50, addr } = match;
  return new Promise<T>((resolve, reject) => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const cleanup = () => {
      device.removeEventListener("inputreport", onReport as EventListener);
      if (timer !== undefined) {
        clearTimeout(timer);
        timer = undefined;
      }
    };
    timer = setTimeout(() => {
      cleanup();
      reject(new Error(`Command 0x${command.toString(16)} response timeout`)); // 命令...响应超时
    }, timeout);
    const onReport = (event: HIDInputReportEvent) => {
      const raw = new Uint8Array(event.data.buffer);
      const parsed = parseResponse(raw);
      if (!parsed || parsed.cmd !== command) return;
      if (addr !== undefined && parsed.addr !== addr) return;
      cleanup();
      try {
        resolve(transform(raw));
      } catch (err) {
        reject(err);
      }
    };
    device.addEventListener("inputreport", onReport as EventListener);
  });
}

export interface TransferOptions {
  cmd: CMD;
  /** Total content bytes to transfer across all packets (default 24). */
  contentSize?: number;
  /** Starting address (default 0). */
  addrStart?: number;
  /** Outgoing payload for writes; omit for reads. */
  data?: Uint8Array;
  /** Per-packet response timeout in ms (default 500). */
  timeout?: number;
  /** Retries per packet before giving up (default 3). */
  maxRetries?: number;
  /** Header byte count reserved per packet (default 8). */
  headerCount?: number;
  otherHeader?: number[];
  customHeader?: Uint8Array;
  /** Match responses against a different opcode than `cmd`. */
  responseCmd?: CMD;
  abortSignal?: AbortSignal;
  /** Set last-packet flag on the final chunk (default true). */
  isNeedLastPacketFlag?: boolean;
  /** Require the response addr to match the request addr (default false). */
  checkAddr?: boolean;
}

/**
 * Chunked request/response transport. Minified `C`, exported as `readDataChunks`.
 *
 * Splits `contentSize` into ceil(contentSize / (reportSize - headerCount))
 * packets, sends each with `sendReport(REPORT_ID, …)`, and awaits a matching
 * response with retries. Returns the array of raw response packets; callers
 * typically do `responses.flatMap(r => [...r.slice(8)]).slice(0, contentSize)`
 * to reassemble the payload.
 *
 * NOTE: the upstream version, on terminal failure, tears down the device and
 * navigates the SPA back to "/". That UI coupling is intentionally omitted
 * here — this layer just throws.
 */
export async function transfer(device: HIDDevice, opts: TransferOptions): Promise<Uint8Array[]> {
  const {
    cmd,
    contentSize = 24,
    addrStart = 0,
    data,
    timeout = 500,
    maxRetries = 3,
    headerCount = HEADER_SIZE,
    otherHeader,
    customHeader,
    responseCmd = cmd,
    abortSignal,
    isNeedLastPacketFlag = true,
    checkAddr = false,
  } = opts;

  const reportSize =
    device?.collections?.[0]?.outputReports?.[0]?.items?.[0]?.reportCount || DEFAULT_REPORT_SIZE;
  const perPacket = reportSize - headerCount;
  const packetCount = Math.ceil(contentSize / perPacket);
  const responses: Uint8Array[] = [];

  for (let i = 0; i < packetCount; i++) {
    if (abortSignal?.aborted) throw new Error("Operation aborted"); // 操作已中断

    const addr = addrStart + i * perPacket;
    const remaining = contentSize - i * perPacket;
    const isLast = isNeedLastPacketFlag ? i === packetCount - 1 : false;
    const len = i === packetCount - 1 ? remaining : perPacket;

    let chunk: Uint8Array | undefined;
    if (data) {
      const from = i * perPacket;
      chunk = data.slice(from, Math.min(from + perPacket, data.length));
    }

    const packet = buildPacket(cmd, len, addr, chunk, reportSize, {
      otherHeader,
      customHeader,
      isLastPacket: isLast,
    });

    let attempt = 0;
    let response: Uint8Array | null = null;
    while (attempt <= maxRetries && !response) {
      if (abortSignal?.aborted) throw new Error("Operation aborted");
      try {
        const pending = waitForResponse(device, {
          command: responseCmd,
          timeout,
          addr: checkAddr ? addr : undefined,
        });
        await device.sendReport(REPORT_ID, packet);
        response = await pending;
      } catch (err) {
        if (maxRetries === 0) return [];
        attempt++;
        if (attempt <= maxRetries) {
          console.warn(`Packet 0x${cmd.toString(16)} timed out, resend #${attempt}`);
        } else {
          throw new Error(`Packet 0x${cmd.toString(16)} no response after ${maxRetries} retries: ${err}`);
        }
      }
    }
    if (response) responses.push(response);
  }
  return responses;
}

/** Reassemble a chunked read into a flat payload of `contentSize` bytes. */
export function concatPayload(responses: Uint8Array[], contentSize: number): Uint8Array {
  return new Uint8Array(responses.flatMap((r) => Array.from(r.slice(HEADER_SIZE)))).slice(0, contentSize);
}

// ─── Device open ──────────────────────────────────────────────────────────────

/** Find & open a device by productId on a given usagePage. Minified `z`. */
export async function openDevice(
  productId: number,
  usagePage = DEFAULT_USAGE_PAGE,
): Promise<HIDDevice | false> {
  const devices = await navigator.hid.getDevices();
  const device = devices.find((d) =>
    d.collections.some((c) => c.usagePage === usagePage && d.productId === productId),
  );
  if (!device) {
    console.warn(`No device found with usagePage ${usagePage}`);
    return false;
  }
  if (!device.opened) await device.open();
  return device;
}

// ─── Worked example: GET_DEVICE_INFO ──────────────────────────────────────────
// Minified `Ce` (exported `getDeviceInfo`). 48-byte payload layout, verbatim.

export interface DeviceInfo {
  romSize: number;
  macroSpaceSize: number;
  vid: number;
  pid: number;
  /** Firmware version, e.g. 1.23 (BCD-ish: low nibble + high nibble*10 + byte9*100, /100). */
  version: number;
  sensor: number;
  manufacturer: number;
  product: number;
  workMode: number;
  batteryLevel: number;
  chargeStatus: number;
  currentProfile: number;
  axisInfo: number;
  tftMaxFrames: number;
  gifMaxFrames: number;
  ledMaxFrames: number;
  tftDirection: number;
  rtPrecision: number;
  frameVersion: number;
  lightingVersion: number;
}

export async function getDeviceInfo(device: HIDDevice): Promise<DeviceInfo> {
  const responses = await transfer(device, { cmd: CMD.GET_DEVICE_INFO, contentSize: 48 });
  const e = concatPayload(responses, 48);
  return {
    romSize: e[0],
    macroSpaceSize: 512, // upstream conditionally reads e[2]|e[3]<<8 when macroConfig.isGetMaxMacroSpaceSize
    vid: e[4] | (e[5] << 8),
    pid: e[6] | (e[7] << 8),
    version: parseFloat((((e[8] & 15) + ((e[8] & 240) >> 4) * 10 + e[9] * 100) / 100).toFixed(2)),
    sensor: e[10] | (e[11] << 8),
    manufacturer: e[12] | (e[13] << 8),
    product: e[14] | (e[15] << 8),
    workMode: e[16],
    batteryLevel: e[17],
    chargeStatus: e[18],
    currentProfile: e[19],
    axisInfo: e[20] | (e[21] << 8),
    tftMaxFrames: e[22] | (e[23] << 8),
    gifMaxFrames: e[24] | (e[25] << 8),
    ledMaxFrames: e[26] | (e[27] << 8),
    tftDirection: e[28],
    rtPrecision: e[29],
    frameVersion: e[30],
    lightingVersion: e[31],
  };
}
