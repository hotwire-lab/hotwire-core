// HotWire Core — Unified Web Serial, WebUSB (CDC-ACM, FTDI, CP210x, CH34x, PL2303),
// WakeLock, and esptool-js Web Flasher bridge for Rust/WASM.

const WEBUSB_FILTERS = [
  { vendorId: 0x0403 }, // FTDI
  { vendorId: 0x10c4 }, // Silicon Labs CP210x
  { vendorId: 0x1a86 }, // QinHeng CH340/CH341/CH9102
  { vendorId: 0x067b }, // Prolific PL2303
  { vendorId: 0x303a }, // Espressif
  { vendorId: 0x2e8a }, // Raspberry Pi Pico
  { vendorId: 0x0483 }, // STMicroelectronics
  { vendorId: 0x1915 }, // Nordic
  { vendorId: 0x2341 }, // Arduino
  { vendorId: 0x1d50 }, // OpenMoko / Black Magic Probe
  { vendorId: 0x239a }, // Adafruit
  { vendorId: 0x1b4f }, // SparkFun
  { vendorId: 0x2886 }, // Seeed Studio
  { classCode: 2, subclassCode: 2 },
  { classCode: 239, subclassCode: 2, protocolCode: 1 },
];

const NATIVE_CDC_VIDS = new Set([0x303a, 0x2e8a, 0x1915, 0x2341, 0x1d50, 0x239a, 0x2886]);

function formatVidPid(vid, pid) {
  if (!vid && !pid) return '';
  return `${(vid ?? 0).toString(16).toUpperCase().padStart(4, '0')}:${(pid ?? 0)
    .toString(16)
    .toUpperCase()
    .padStart(4, '0')}`;
}

function detectDriverKind(vid) {
  switch (vid) {
    case 0x0403: return 'FTDI';
    case 0x10c4: return 'CP210X';
    case 0x1a86: return 'CH34X';
    case 0x067b: return 'PL2303';
    default: return 'CDC_ACM';
  }
}

function identifyChip(vid) {
  if (!vid) return 'Web Serial';
  switch (vid) {
    case 0x0403: return 'FTDI';
    case 0x10c4: return 'CP210x';
    case 0x1a86: return 'CH34x';
    case 0x067b: return 'PL2303';
    case 0x303a: return 'Espressif CDC';
    case 0x2e8a: return 'RP2040 CDC';
    case 0x0483: return 'STM32 VCP';
    case 0x1915: return 'Nordic CDC';
    case 0x2341: return 'Arduino CDC';
    case 0x2886: return 'Seeed XIAO';
    default: return 'CDC-ACM';
  }
}

function resolveInitialLines(vid, driverKind, cfg) {
  if (typeof cfg.forceDtr === 'boolean' && typeof cfg.forceRts === 'boolean') {
    return { dtr: cfg.forceDtr, rts: cfg.forceRts };
  }
  const isNative = NATIVE_CDC_VIDS.has(vid) || (vid === 0x0483 && (!driverKind || driverKind === 'CDC_ACM'));
  return { dtr: isNative, rts: false };
}

function encodeCdcLineCoding(cfg) {
  const buf = new ArrayBuffer(7);
  const v = new DataView(buf);
  v.setUint32(0, cfg.baudRate || 115200, true);
  v.setUint8(4, cfg.stopBits === 2 ? 2 : 0);
  v.setUint8(5, cfg.parity || 0);
  v.setUint8(6, cfg.dataBits || 8);
  return new Uint8Array(buf);
}

function encodeFtdiBaud(baud) {
  if (baud >= 3000000) return { value: 0, index: 0 };
  if (baud === 2000000) return { value: 1, index: 0 };
  const sub = [0, 3, 2, 4, 1, 5, 6, 7];
  const div = Math.max(2, Math.round((48000000 / 16) / baud * 8));
  let enc = (div >> 3) | (sub[div & 7] << 14);
  if (enc === 1) enc = 0;
  else if (enc === 0x4001) enc = 1;
  return { value: enc & 0xffff, index: (enc >> 16) & 0xffff };
}

function encodeCh34xBaud(baud) {
  const clk = 12000000;
  const bClamp = Math.max(50, Math.min(3000000, baud));
  let a = 0;
  let b = 0;
  if (bClamp >= 1500000) {
    a = 3;
    b = 256 - Math.round(clk / bClamp);
  } else if (bClamp >= 24000) {
    a = 3;
    b = 256 - Math.round((clk / 2) / bClamp);
    if (b < 2 || b > 255) {
      a = 2;
      b = 256 - Math.round((clk / 16) / bClamp);
    }
  } else if (bClamp >= 3000) {
    a = 2;
    b = 256 - Math.round((clk / 16) / bClamp);
  } else if (bClamp >= 400) {
    a = 1;
    b = 256 - Math.round((clk / 128) / bClamp);
  } else {
    a = 0;
    b = 256 - Math.round((clk / 1024) / bClamp);
  }
  const cb = Math.max(0, Math.min(255, b));
  return { factor: (cb << 8) | 0x80 | a };
}

function decodeFtdiPayload(raw, packetSize = 64) {
  const eff = Math.max(3, packetSize);
  if (raw.byteLength <= 2) return new Uint8Array(0);
  if (raw.byteLength <= eff) return raw.subarray(2);
  const chunks = [];
  let total = 0;
  for (let off = 0; off < raw.byteLength; off += eff) {
    const end = Math.min(raw.byteLength, off + eff);
    if (end - off > 2) {
      const s = raw.subarray(off + 2, end);
      chunks.push(s);
      total += s.byteLength;
    }
  }
  const out = new Uint8Array(total);
  let pos = 0;
  for (const c of chunks) {
    out.set(c, pos);
    pos += c.byteLength;
  }
  return out;
}

function enumerateWebUsbPorts(device) {
  const config = device.configuration ?? device.configurations?.[0];
  if (!config) return [];
  const vendorKind = detectDriverKind(device.vendorId);
  const ifaces = config.interfaces;
  const ports = [];
  const isKnownBridge = device.vendorId in { 0x0403: 1, 0x10c4: 1, 0x1a86: 1, 0x067b: 1 };

  for (let i = 0; i < ifaces.length; i++) {
    const iface = ifaces[i];
    const alt = iface.alternate;
    let inEp = null;
    let outEp = null;
    for (const ep of alt.endpoints) {
      if (ep.type !== 'bulk') continue;
      if (ep.direction === 'in' && !inEp) inEp = ep;
      if (ep.direction === 'out' && !outEp) outEp = ep;
    }
    if (!inEp || !outEp) continue;

    let ctrlIface = iface.interfaceNumber;
    let kind = vendorKind;
    if (alt.interfaceClass === 10) {
      kind = 'CDC_ACM';
      for (let j = i - 1; j >= 0; j--) {
        if (ifaces[j].alternate.interfaceClass === 2) {
          ctrlIface = ifaces[j].interfaceNumber;
          break;
        }
      }
    } else if (alt.interfaceClass === 2 && alt.interfaceSubclass === 2) {
      kind = 'CDC_ACM';
    }

    const isCdc = alt.interfaceClass === 10 || alt.interfaceClass === 2;
    if (!isKnownBridge && !isCdc && alt.interfaceClass === 0xff && ports.length > 0) continue;

    ports.push({
      portIndex: ports.length,
      driverKind: kind,
      controlInterfaceNumber: ctrlIface,
      dataInterfaceNumber: iface.interfaceNumber,
      inEndpointNumber: inEp.endpointNumber,
      outEndpointNumber: outEp.endpointNumber,
      inPacketSize: inEp.packetSize || 64,
    });
  }
  return ports;
}

class WebUsbDriverSession {
  constructor(device, port) {
    this.device = device;
    this.port = port;
  }

  async init(cfg, dtr, rts) {
    if (this.port.driverKind === 'FTDI') {
      await this.device.controlTransferOut({
        requestType: 'vendor', recipient: 'device', request: 0x00, value: 0x00,
        index: this.port.dataInterfaceNumber + 1,
      });
    } else if (this.port.driverKind === 'CP210X') {
      await this.device.controlTransferOut({
        requestType: 'vendor', recipient: 'interface', request: 0x00, value: 0x0001,
        index: this.port.dataInterfaceNumber,
      });
    } else if (this.port.driverKind === 'CH34X') {
      await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'device', request: 0xa1, value: 0, index: 0 });
      await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'device', request: 0x9a, value: 0x0f2c, index: 0x0007 });
    }
    try { await this.setParameters(cfg); } catch {}
    try { await this.setControlLines(dtr, rts); } catch {}
  }

  async setParameters(cfg) {
    switch (this.port.driverKind) {
      case 'CDC_ACM':
        await this.device.controlTransferOut(
          { requestType: 'class', recipient: 'interface', request: 0x20, value: 0, index: this.port.controlInterfaceNumber },
          encodeCdcLineCoding(cfg),
        );
        break;
      case 'FTDI': {
        const chIdx = this.port.dataInterfaceNumber + 1;
        const { value, index } = encodeFtdiBaud(cfg.baudRate);
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'device', request: 0x03, value, index: (index << 8) | chIdx });
        const fPar = cfg.parity === 1 ? 0x0100 : cfg.parity === 2 ? 0x0200 : 0x0000;
        const fStop = cfg.stopBits === 2 ? 0x1000 : 0x0000;
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'device', request: 0x04, value: (cfg.dataBits & 0xff) | fPar | fStop, index: chIdx });
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'device', request: 0x02, value: cfg.flowControl === 1 ? 0x0100 : 0x0000, index: chIdx });
        break;
      }
      case 'CP210X': {
        const buf = new ArrayBuffer(4);
        new DataView(buf).setUint32(0, cfg.baudRate, true);
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'interface', request: 0x1e, value: 0, index: this.port.dataInterfaceNumber }, buf);
        const cPar = cfg.parity === 1 ? 0x0010 : cfg.parity === 2 ? 0x0020 : 0x0000;
        const cStop = cfg.stopBits === 2 ? 0x0004 : 0x0000;
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'interface', request: 0x03, value: ((cfg.dataBits & 0x0f) << 8) | cPar | cStop, index: this.port.dataInterfaceNumber });
        break;
      }
      case 'CH34X': {
        const { factor } = encodeCh34xBaud(cfg.baudRate);
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'device', request: 0x9a, value: 0x1312, index: factor });
        let lcr = 0xc0 | (cfg.dataBits === 7 ? 0x02 : 0x03);
        if (cfg.stopBits === 2) lcr |= 0x04;
        if (cfg.parity === 1) lcr |= 0x08;
        if (cfg.parity === 2) lcr |= 0x18;
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'device', request: 0x9a, value: 0x2518, index: lcr });
        break;
      }
      case 'PL2303':
        await this.device.controlTransferOut(
          { requestType: 'class', recipient: 'interface', request: 0x20, value: 0, index: 0 },
          encodeCdcLineCoding(cfg),
        );
        break;
    }
  }

  async setControlLines(dtr, rts) {
    switch (this.port.driverKind) {
      case 'CDC_ACM':
      case 'PL2303':
        await this.device.controlTransferOut({ requestType: 'class', recipient: 'interface', request: 0x22, value: (dtr ? 1 : 0) | (rts ? 2 : 0), index: this.port.controlInterfaceNumber });
        break;
      case 'FTDI':
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'device', request: 0x01, value: (dtr ? 0x0101 : 0x0100) | (rts ? 0x0202 : 0x0200), index: this.port.dataInterfaceNumber + 1 });
        break;
      case 'CP210X':
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'interface', request: 0x07, value: 0x0300 | (dtr ? 1 : 0) | (rts ? 2 : 0), index: this.port.dataInterfaceNumber });
        break;
      case 'CH34X':
        await this.device.controlTransferOut({ requestType: 'vendor', recipient: 'device', request: 0xa4, value: (~((dtr ? 0x20 : 0) | (rts ? 0x40 : 0))) & 0xff, index: 0 });
        break;
    }
  }

  decodeIncoming(raw) {
    return this.port.driverKind === 'FTDI' ? decodeFtdiPayload(raw, this.port.inPacketSize) : raw;
  }
}

class WebUsbCdcFlashPort {
  constructor(device) {
    this.device = device;
    this.readable = null;
    this.writable = null;
    this.desc = null;
    this.currentDtr = false;
    this.currentRts = false;
    this.readLoopRunning = false;
    this.controlQueue = Promise.resolve();
  }

  getInfo() {
    return { usbVendorId: this.device.vendorId, usbProductId: this.device.productId };
  }

  addEventListener() {}
  removeEventListener() {}

  async open(options = { baudRate: 115200 }) {
    if (this.device.opened) await this.close();
    await this.device.open();
    if (!this.device.configuration) await this.device.selectConfiguration(1);
    const ports = enumerateWebUsbPorts(this.device);
    const desc = ports[0];
    if (!desc) {
      await this.device.close();
      throw new Error('No USB CDC-ACM bulk endpoints found on selected device.');
    }
    if (desc.controlInterfaceNumber !== desc.dataInterfaceNumber) {
      try { await this.device.claimInterface(desc.controlInterfaceNumber); } catch {}
    }
    await this.device.claimInterface(desc.dataInterfaceNumber);
    this.desc = desc;
    this.currentDtr = false;
    this.currentRts = false;
    await this.setBaudRate(options.baudRate || 115200);
    await this.setSignals({ dataTerminalReady: false, requestToSend: false });
    this.createStreams();
  }

  async setBaudRate(baudRate) {
    if (!this.desc || !this.device.opened) return;
    const idx = this.desc.controlInterfaceNumber;
    this.controlQueue = this.controlQueue
      .catch(() => {})
      .then(async () => {
        if (!this.device.opened) return;
        try {
          await this.device.controlTransferOut(
            { requestType: 'class', recipient: 'interface', request: 0x20, value: 0, index: idx },
            encodeCdcLineCoding({ baudRate, dataBits: 8, stopBits: 1, parity: 0 }),
          );
        } catch {}
      });
    return this.controlQueue;
  }

  setSignals(signals = {}) {
    if (!this.desc || !this.device.opened) return Promise.resolve();
    // Preserve whichever line esptool-js omitted (essential for UsbJtagSerialReset!)
    if (signals.dataTerminalReady !== undefined) {
      this.currentDtr = Boolean(signals.dataTerminalReady);
    }
    if (signals.requestToSend !== undefined) {
      this.currentRts = Boolean(signals.requestToSend);
    }
    const value = (this.currentDtr ? 1 : 0) | (this.currentRts ? 2 : 0);
    const index = this.desc.controlInterfaceNumber;
    this.controlQueue = this.controlQueue
      .catch(() => {})
      .then(async () => {
        if (!this.device.opened) return;
        await this.device.controlTransferOut({
          requestType: 'class',
          recipient: 'interface',
          request: 0x22,
          value,
          index,
        });
      })
      .catch(() => {});
    return this.controlQueue;
  }

  createStreams() {
    const device = this.device;
    const desc = this.desc;
    // Use exact maxPacketSize (64 B on Full-Speed CDC) so SLIP replies that are
    // an exact multiple of 64 B never stall waiting for a Zero-Length Packet (ZLP).
    const readSize = Math.max(64, desc.inPacketSize || 64);
    this.readLoopRunning = true;
    const self = this;

    this.readable = new ReadableStream({
      async pull(controller) {
        while (self.readLoopRunning && device.opened) {
          try {
            const res = await device.transferIn(desc.inEndpointNumber, readSize);
            if (!self.readLoopRunning) break;
            if (res.status === 'stall') {
              try { await device.clearHalt('in', desc.inEndpointNumber); } catch {}
              continue;
            }
            if (res.status === 'ok' && res.data && res.data.byteLength > 0) {
              controller.enqueue(new Uint8Array(res.data.buffer, res.data.byteOffset, res.data.byteLength));
              return;
            }
          } catch {
            break;
          }
        }
        try { controller.close(); } catch {}
      },
      cancel() {
        self.readLoopRunning = false;
      },
    });

    this.writable = new WritableStream({
      async write(chunk) {
        if (!device.opened) throw new Error('USB CDC device is not open');
        const bytes = chunk instanceof Uint8Array ? chunk : new Uint8Array(chunk);
        // Chunk writes to <= 2048 B to avoid Android WebUSB bulkTransfer buffer limits
        const MAX_CHUNK = 2048;
        for (let off = 0; off < bytes.byteLength; off += MAX_CHUNK) {
          const slice = bytes.slice(off, Math.min(bytes.byteLength, off + MAX_CHUNK));
          let res = await device.transferOut(desc.outEndpointNumber, slice);
          if (res.status === 'stall') {
            try { await device.clearHalt('out', desc.outEndpointNumber); } catch {}
            res = await device.transferOut(desc.outEndpointNumber, slice);
          }
          if (res.status !== 'ok') {
            throw new Error(`USB transferOut failed: ${res.status}`);
          }
        }
      },
    });
  }

  async close() {
    this.readLoopRunning = false;
    this.readable = null;
    this.writable = null;
    const desc = this.desc;
    this.desc = null;
    if (this.device.opened) {
      if (desc) {
        try {
          await this.device.controlTransferOut({
            requestType: 'class',
            recipient: 'interface',
            request: 0x22,
            value: 0,
            index: desc.controlInterfaceNumber,
          });
        } catch {}
        try { await this.device.releaseInterface(desc.dataInterfaceNumber); } catch {}
        if (desc.controlInterfaceNumber !== desc.dataInterfaceNumber) {
          try { await this.device.releaseInterface(desc.controlInterfaceNumber); } catch {}
        }
      }
      try { await this.device.close(); } catch {}
    }
  }
}

const state = {
  webSerialPort: null,
  webSerialReader: null,
  webSerialWriter: null,
  webUsbDevice: null,
  webUsbPorts: [],
  webUsbPortIdx: 0,
  webUsbSession: null,
  webUsbReadActive: false,
  wakeLock: null,
  epoch: 0,
  cfg: {
    baudRate: 921600,
    dataBits: 8,
    stopBits: 1,
    parity: 0,
    flowControl: 0,
    bufferSize: 131072,
    forceDtr: true,
    forceRts: false,
  },
  initCmds: new Uint8Array(0),
  storageKey: 'hotwire_last_device',
  dtr: true,
  rts: false,
  status: 'waiting',
  transport: 'none',
  deviceName: '',
  vidPid: '',
  portIndex: 0,
  portCount: 0,
  autoConnectEnabled: true,
  manualDisconnect: false,
  demoActive: false,
  flashing: false,
  reconnecting: false,
  reconnectTimer: null,
  listenersRegistered: false,
  handshakeOk: false,
  writeQueue: Promise.resolve(),
  onState: null,
  onBytes: null,
};

function parseConfigJson(cfgJson) {
  if (!cfgJson) return { ...state.cfg };
  try {
    const parsed = JSON.parse(cfgJson);
    return {
      baudRate: parsed.baud_rate ?? parsed.baudRate ?? 921600,
      dataBits: parsed.data_bits ?? parsed.dataBits ?? 8,
      stopBits: parsed.stop_bits ?? parsed.stopBits ?? 1,
      parity: parsed.parity ?? 0,
      flowControl: parsed.flow_control ?? parsed.flowControl ?? 0,
      bufferSize: parsed.buffer_size ?? parsed.bufferSize ?? 131072,
      forceDtr: parsed.force_dtr ?? parsed.forceDtr ?? null,
      forceRts: parsed.force_rts ?? parsed.forceRts ?? null,
      autoConnect: parsed.auto_connect ?? parsed.autoConnect,
      storageKey: parsed.storage_key ?? parsed.storageKey,
    };
  } catch {
    return { ...state.cfg };
  }
}

function saveLastDevice(backend, vid, pid, serialNumber = '') {
  try {
    localStorage.setItem(
      state.storageKey,
      JSON.stringify({ backend, vid: vid ?? 0, pid: pid ?? 0, serialNumber: serialNumber ?? '' }),
    );
  } catch {}
}

function loadLastDevice() {
  try {
    const raw = localStorage.getItem(state.storageKey);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function matchesLastSerialPort(port, last) {
  if (!last || last.backend !== 'webserial') return false;
  const info = port.getInfo?.() ?? {};
  const vid = info.usbVendorId ?? 0;
  const pid = info.usbProductId ?? 0;
  if (last.vid === 0 && last.pid === 0) return true;
  return vid === last.vid && pid === last.pid;
}

function matchesLastUsbDevice(device, last) {
  if (!last || last.backend !== 'webusb') return false;
  if ((device.vendorId ?? 0) !== last.vid || (device.productId ?? 0) !== last.pid) return false;
  if (last.serialNumber && device.serialNumber && device.serialNumber !== last.serialNumber) {
    return false;
  }
  return true;
}

function preferredBackend() {
  if (typeof navigator === 'undefined') return 'none';
  const ua = navigator.userAgent ?? '';
  const isMobile = /Android|iPhone|iPad|iPod|Mobile/i.test(ua);
  const hasSerial = 'serial' in navigator && Boolean(navigator.serial);
  const hasUsb = 'usb' in navigator && Boolean(navigator.usb);
  if (isMobile && hasUsb) return 'webusb';
  if (hasSerial) return 'webserial';
  if (hasUsb) return 'webusb';
  return 'none';
}

function notifyState(status, transport, deviceName, vidPid, portIdx, portCount, dtr, rts, errorMessage) {
  state.status = status;
  state.transport = transport;
  state.deviceName = deviceName ?? '';
  state.vidPid = vidPid ?? '';
  state.portIndex = portIdx ?? 0;
  state.portCount = portCount ?? 0;
  state.dtr = Boolean(dtr);
  state.rts = Boolean(rts);
  if (state.onState) {
    state.onState(
      JSON.stringify({
        status,
        transport,
        device_name: state.deviceName,
        vid_pid: state.vidPid,
        port_index: state.portIndex,
        port_count: state.portCount,
        dtr: state.dtr,
        rts: state.rts,
        error_message: errorMessage ?? '',
      }),
    );
  }
}

// Re-emits the current connected snapshot (used after DTR/RTS changes so the
// Rust side and UI stay in sync with the physical line state).
function notifyCurrentState() {
  if (state.status === 'connected') {
    notifyState(
      state.status,
      state.transport,
      state.deviceName,
      state.vidPid,
      state.portIndex,
      state.portCount,
      state.dtr,
      state.rts,
      '',
    );
  }
}

async function acquireWakeLock() {
  try {
    if ('wakeLock' in navigator && (!state.wakeLock || state.wakeLock.released)) {
      state.wakeLock = await navigator.wakeLock.request('screen');
    }
  } catch {}
}

async function releaseWakeLock() {
  try {
    if (state.wakeLock) {
      if (!state.wakeLock.released) await state.wakeLock.release();
      state.wakeLock = null;
    }
  } catch {}
}

async function closeAll(preserveSerialPortRef = false) {
  state.webUsbReadActive = false;
  if (state.webSerialReader) {
    try { await state.webSerialReader.cancel(); } catch {}
    try { state.webSerialReader.releaseLock(); } catch {}
    state.webSerialReader = null;
  }
  if (state.webSerialWriter) {
    try { state.webSerialWriter.releaseLock(); } catch {}
    state.webSerialWriter = null;
  }
  if (state.webSerialPort) {
    const port = state.webSerialPort;
    if (!preserveSerialPortRef) state.webSerialPort = null;
    try { await port.setSignals({ dataTerminalReady: false, requestToSend: false }); } catch {}
    try { await port.close(); } catch {}
  }
  if (state.webUsbDevice) {
    const dev = state.webUsbDevice;
    const ports = state.webUsbPorts;
    const activeSession = state.webUsbSession;
    state.webUsbDevice = null;
    state.webUsbPorts = [];
    state.webUsbSession = null;
    try {
      if (activeSession && dev.opened) {
        try { await activeSession.setControlLines(false, false); } catch {}
      }
      for (const p of ports) {
        try { await dev.releaseInterface(p.dataInterfaceNumber); } catch {}
        if (p.controlInterfaceNumber !== p.dataInterfaceNumber) {
          try { await dev.releaseInterface(p.controlInterfaceNumber); } catch {}
        }
      }
      if (dev.opened) await dev.close();
    } catch {}
  }
  await releaseWakeLock();
}

async function sendInitCommands(epoch, initialDelayMs = 0) {
  const cmds = state.initCmds;
  if (!cmds || cmds.byteLength === 0) return;
  if (initialDelayMs > 0) {
    await new Promise((r) => setTimeout(r, initialDelayMs));
    if (epoch !== state.epoch || state.status !== 'connected' || state.handshakeOk) return;
  }
  try { await js_write_bytes(cmds); } catch {}
  if (typeof window !== 'undefined') {
    for (const delay of [400, 1000, 1800, 2800]) {
      window.setTimeout(() => {
        if (epoch === state.epoch && state.status === 'connected' && !state.handshakeOk) {
          void js_write_bytes(cmds);
        }
      }, delay);
    }
  }
}

async function waitForFreshUsbDevice(
  vid,
  pid,
  serialNumber,
  timeoutMs = 3500,
  excludeDevice = null,
  preferFreshMs = 2200,
) {
  if (typeof navigator === 'undefined' || !navigator.usb) return null;
  const started = Date.now();
  const deadline = started + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const devices = await navigator.usb.getDevices();
      const elapsed = Date.now() - started;
      const pool =
        excludeDevice && elapsed < preferFreshMs
          ? devices.filter((d) => d !== excludeDevice)
          : devices;
      const exact = pool.find(
        (d) =>
          (!vid || d.vendorId === vid) &&
          (!pid || d.productId === pid) &&
          (!serialNumber || !d.serialNumber || d.serialNumber === serialNumber),
      );
      if (exact) return exact;
      const sameVendor = pool.find((d) => vid && d.vendorId === vid);
      if (sameVendor) return sameVendor;
      if (!vid && pool.length > 0) return pool[0];
    } catch {}
    await new Promise((r) => setTimeout(r, 200));
  }
  return null;
}

async function waitForFreshSerialPort(vid, pid, timeoutMs = 3000) {
  if (typeof navigator === 'undefined' || !navigator.serial) return null;
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const ports = await navigator.serial.getPorts();
      const match = ports.find((p) => {
        const info = p.getInfo?.() ?? {};
        return (!vid || info.usbVendorId === vid) && (!pid || info.usbProductId === pid);
      });
      if (match) return match;
      if (!vid && ports.length > 0) return ports[0];
    } catch {}
    await new Promise((r) => setTimeout(r, 200));
  }
  return null;
}

async function openWebSerial(port, cfg) {
  state.epoch += 1;
  state.handshakeOk = false;
  const epoch = state.epoch;
  await closeAll(false);
  state.cfg = { ...state.cfg, ...cfg };

  const info = port.getInfo?.() ?? {};
  const vid = info.usbVendorId ?? 0;
  const pid = info.usbProductId ?? 0;
  const lines = resolveInitialLines(vid, null, state.cfg);
  state.dtr = lines.dtr;
  state.rts = lines.rts;
  const label = identifyChip(vid);
  const vp = formatVidPid(vid, pid);

  notifyState('connecting', 'webserial', label, vp, 0, 1, state.dtr, state.rts, '');

  const parityStr = state.cfg.parity === 1 ? 'odd' : state.cfg.parity === 2 ? 'even' : 'none';
  try {
    await port.open({
      baudRate: state.cfg.baudRate,
      dataBits: state.cfg.dataBits,
      stopBits: state.cfg.stopBits,
      parity: parityStr,
      flowControl: state.cfg.flowControl === 1 ? 'hardware' : 'none',
      bufferSize: state.cfg.bufferSize || 131072,
    });
    if (epoch !== state.epoch) {
      try { await port.close(); } catch {}
      return false;
    }
    state.webSerialPort = port;
    if (port.writable) state.webSerialWriter = port.writable.getWriter();
    state.manualDisconnect = false;
    state.demoActive = false;
    saveLastDevice('webserial', vid, pid, '');
    try { await port.setSignals({ dataTerminalReady: state.dtr, requestToSend: state.rts }); } catch {}
    await acquireWakeLock();
    notifyState('connected', 'webserial', label, vp, 0, 1, state.dtr, state.rts, '');

    (async () => {
      try {
        while (state.epoch === epoch && port.readable) {
          const reader = port.readable.getReader();
          state.webSerialReader = reader;
          try {
            while (state.epoch === epoch) {
              const { value, done } = await reader.read();
              if (done) break;
              if (value && value.byteLength > 0 && state.onBytes) state.onBytes(value);
            }
          } finally {
            try { reader.releaseLock(); } catch {}
            if (state.webSerialReader === reader) state.webSerialReader = null;
          }
        }
      } catch (err) {
        if (state.epoch === epoch) {
          await closeAll(false);
          notifyState('waiting', 'none', '', '', 0, 0, false, false, err?.message ?? 'Serial disconnected');
        }
        return;
      }
      if (state.epoch === epoch) {
        await closeAll(false);
        notifyState('waiting', 'none', '', '', 0, 0, false, false, '');
      }
    })();

    await sendInitCommands(epoch);
    return true;
  } catch (err) {
    await closeAll(false);
    const msg = err instanceof Error ? err.message : String(err);
    notifyState('waiting', 'none', '', '', 0, 0, false, false, `Failed to open serial port: ${msg}`);
    return false;
  }
}

async function openWebUsb(device, cfg, portIdx = 0) {
  state.epoch += 1;
  state.handshakeOk = false;
  const epoch = state.epoch;
  await closeAll(false);
  state.cfg = { ...state.cfg, ...cfg };

  const vid = device.vendorId ?? 0;
  const pid = device.productId ?? 0;
  const name = device.productName?.trim() || identifyChip(vid);
  const vp = formatVidPid(vid, pid);

  notifyState('connecting', 'webusb', name, vp, portIdx, 1, false, false, '');

  try {
    if (!device.opened) await device.open();
    if (!device.configuration) await device.selectConfiguration(1);
    if (epoch !== state.epoch) {
      try { if (device.opened) await device.close(); } catch {}
      return false;
    }

    const ports = enumerateWebUsbPorts(device);
    if (ports.length === 0) {
      await device.close();
      notifyState('waiting', 'none', '', '', 0, 0, false, false, 'No supported USB serial endpoints found.');
      return false;
    }

    const idx = Math.max(0, Math.min(portIdx, ports.length - 1));
    const desc = ports[idx];
    // Claim CDC control interface BEFORE data interface so SET_LINE_CODING / SET_CONTROL_LINE_STATE succeed on Android.
    // Do NOT call clearHalt() unconditionally on healthy endpoints: CLEAR_FEATURE(ENDPOINT_HALT)
    // can desync DATA0/DATA1 toggles on Android usbfs + ESP32 native USB-Serial-JTAG.
    if (desc.controlInterfaceNumber !== desc.dataInterfaceNumber) {
      try { await device.claimInterface(desc.controlInterfaceNumber); } catch {}
    }
    await device.claimInterface(desc.dataInterfaceNumber);
    if (epoch !== state.epoch) {
      try { await device.close(); } catch {}
      return false;
    }

    const lines = resolveInitialLines(vid, desc.driverKind, state.cfg);
    state.dtr = lines.dtr;
    state.rts = lines.rts;
    state.webUsbDevice = device;
    state.webUsbPorts = ports;
    state.webUsbPortIdx = idx;

    const session = new WebUsbDriverSession(device, desc);
    state.webUsbSession = session;
    state.manualDisconnect = false;
    state.demoActive = false;
    saveLastDevice('webusb', vid, pid, device.serialNumber ?? '');
    await session.init(state.cfg, state.dtr, state.rts);
    await acquireWakeLock();
    notifyState('connected', 'webusb', name, vp, idx, ports.length, state.dtr, state.rts, '');

    state.webUsbReadActive = true;
    (async () => {
      // Request exact endpoint maxPacketSize (64 B on Full-Speed CDC) so transfers complete
      // immediately even when ESP32 HWCDC sends an exact multiple of 64 B (e.g. 128 B) without a ZLP.
      const readSize = Math.max(64, desc.inPacketSize || 64);
      try {
        while (state.epoch === epoch && state.webUsbReadActive && device.opened) {
          const res = await device.transferIn(desc.inEndpointNumber, readSize);
          if (!state.webUsbReadActive || state.epoch !== epoch) break;
          if (res.status === 'stall') {
            try { await device.clearHalt('in', desc.inEndpointNumber); } catch {}
            continue;
          }
          if (res.status === 'ok' && res.data && res.data.byteLength > 0) {
            const raw = new Uint8Array(res.data.buffer, res.data.byteOffset, res.data.byteLength);
            const decoded = session.decodeIncoming(raw);
            if (decoded.byteLength > 0 && state.onBytes) state.onBytes(decoded);
          }
        }
      } catch (err) {
        if (state.epoch === epoch) {
          await closeAll(false);
          notifyState('waiting', 'none', '', '', 0, 0, false, false, err?.message ?? 'WebUSB disconnected');
        }
        return;
      }
      if (state.epoch === epoch) {
        await closeAll(false);
        notifyState('waiting', 'none', '', '', 0, 0, false, false, '');
      }
    })();

    // Give ESP32-S3/C6 USB-Serial-JTAG ~70 ms to settle after DTR assertion on Android OTG
    await sendInitCommands(epoch, 70);
    return true;
  } catch (err) {
    await closeAll(false);
    try { if (device.opened) await device.close(); } catch {}
    const msg = err instanceof Error ? err.message : String(err);
    const hint = msg.toLowerCase().includes('claim')
      ? `${msg} (Close Arduino IDE / Serial Monitor or use Web Serial on Desktop.)`
      : msg;
    notifyState('waiting', 'none', '', '', 0, 0, false, false, hint);
    return false;
  }
}

function scheduleAutoReconnect(candidate = null, delayMs = 250, attempt = 0) {
  if (!state.autoConnectEnabled || state.manualDisconnect || state.flashing || state.demoActive) return;
  if (state.webSerialPort || state.webUsbDevice || state.reconnecting) return;
  if (state.reconnectTimer) {
    clearTimeout(state.reconnectTimer);
    state.reconnectTimer = null;
  }

  state.reconnectTimer = setTimeout(async () => {
    state.reconnectTimer = null;
    if (!state.autoConnectEnabled || state.manualDisconnect || state.flashing || state.demoActive) return;
    if (state.webSerialPort || state.webUsbDevice || state.reconnecting) return;

    const last = loadLastDevice();
    const backend = last?.backend || preferredBackend();
    state.reconnecting = true;
    let opened = false;
    let hadTarget = false;
    try {
      if (backend === 'webusb' && navigator.usb) {
        const devices = await navigator.usb.getDevices();
        let target = candidate && typeof candidate.open === 'function' && (!last || matchesLastUsbDevice(candidate, last))
          ? candidate
          : devices.find((d) => matchesLastUsbDevice(d, last));
        if (!target && devices.length > 0) target = devices[0];
        if (target) {
          hadTarget = true;
          opened = await openWebUsb(target, state.cfg, 0);
        }
      } else if (navigator.serial) {
        const ports = await navigator.serial.getPorts();
        let target = candidate && typeof candidate.open === 'function' && (!last || matchesLastSerialPort(candidate, last))
          ? candidate
          : ports.find((p) => matchesLastSerialPort(p, last));
        if (!target && ports.length > 0) target = ports[0];
        if (target) {
          hadTarget = true;
          opened = await openWebSerial(target, state.cfg);
        }
      }
    } catch {
      opened = false;
    } finally {
      state.reconnecting = false;
    }
    if (
      hadTarget &&
      !opened &&
      attempt < 3 &&
      state.autoConnectEnabled &&
      !state.manualDisconnect &&
      !state.flashing
    ) {
      scheduleAutoReconnect(candidate, 450 * (attempt + 1), attempt + 1);
    }
  }, delayMs);
}

// --- Public wasm-bindgen exports ---

export function js_init_transport(onState, onBytes, cfgJson, initCmds) {
  state.onState = onState;
  state.onBytes = onBytes;
  const parsed = parseConfigJson(cfgJson);
  state.cfg = { ...state.cfg, ...parsed };
  if (typeof parsed.autoConnect === 'boolean') {
    state.autoConnectEnabled = parsed.autoConnect;
  }
  if (parsed.storageKey) {
    state.storageKey = parsed.storageKey;
  }
  state.initCmds = initCmds ? new Uint8Array(initCmds) : new Uint8Array(0);

  if (!state.listenersRegistered) {
    state.listenersRegistered = true;
    if (typeof navigator !== 'undefined' && 'serial' in navigator && navigator.serial) {
      navigator.serial.addEventListener('connect', (ev) => {
        scheduleAutoReconnect(ev?.port ?? null, 300);
      });
      navigator.serial.addEventListener('disconnect', (ev) => {
        if (state.transport === 'webserial' && (!ev?.target || ev.target === state.webSerialPort)) {
          state.epoch += 1;
          closeAll(false).then(() => notifyState('waiting', 'none', '', '', 0, 0, false, false, 'Serial port disconnected'));
        }
      });
    }
    if (typeof navigator !== 'undefined' && 'usb' in navigator && navigator.usb) {
      navigator.usb.addEventListener('connect', (ev) => {
        scheduleAutoReconnect(ev?.device ?? null, 300);
      });
      navigator.usb.addEventListener('disconnect', (ev) => {
        if (state.transport === 'webusb' && (!ev?.device || ev.device === state.webUsbDevice)) {
          state.epoch += 1;
          closeAll(false).then(() => notifyState('waiting', 'none', '', '', 0, 0, false, false, 'USB device unplugged'));
        }
      });
    }
    if (typeof window !== 'undefined') {
      window.addEventListener('pagehide', () => {
        state.epoch += 1;
        void closeAll(false);
      });
    }
  }

  if (state.autoConnectEnabled) {
    scheduleAutoReconnect(null, 80);
  }
}

export async function js_connect_usb(cfgJson, initCmds) {
  if (typeof window !== 'undefined' && !window.isSecureContext) {
    notifyState('waiting', 'none', '', '', 0, 0, false, false, 'Web Serial and WebUSB require HTTPS or localhost.');
    return;
  }
  const parsed = parseConfigJson(cfgJson);
  state.cfg = { ...state.cfg, ...parsed };
  if (initCmds) {
    state.initCmds = new Uint8Array(initCmds);
  }
  state.demoActive = false;
  state.manualDisconnect = false;

  const backend = preferredBackend();
  if (backend === 'none') {
    notifyState(
      'waiting', 'none', '', '', 0, 0, false, false,
      'Web Serial / WebUSB is unavailable in this browser. Try Chrome or Edge, or use Demo.',
    );
    return;
  }

  try {
    if (backend === 'webusb' && navigator.usb) {
      const dev = await navigator.usb.requestDevice({ filters: WEBUSB_FILTERS });
      await openWebUsb(dev, state.cfg, 0);
    } else if (navigator.serial) {
      const port = await navigator.serial.requestPort();
      await openWebSerial(port, state.cfg);
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    if (msg.includes('No port selected') || msg.includes('No device selected') || msg.includes('NotFoundError')) {
      notifyState('waiting', 'none', '', '', 0, 0, false, false, '');
    } else {
      notifyState('waiting', 'none', '', '', 0, 0, false, false, msg);
    }
  }
}

export async function js_disconnect_usb() {
  state.manualDisconnect = true;
  if (state.reconnectTimer) {
    clearTimeout(state.reconnectTimer);
    state.reconnectTimer = null;
  }
  // Allow any just-queued STOP / ABORT command to finish flushing before closing USB
  await Promise.race([
    state.writeQueue.catch(() => {}),
    new Promise((r) => setTimeout(r, 120)),
  ]);
  state.epoch += 1;
  await closeAll(false);
  notifyState('waiting', 'none', '', '', 0, 0, false, false, '');
}

export function js_notify_handshake_ok() {
  state.handshakeOk = true;
}

export function js_set_init_commands(initCmds) {
  state.initCmds = initCmds ? new Uint8Array(initCmds) : new Uint8Array(0);
}

export function js_write_bytes(bytes) {
  if (!bytes || bytes.byteLength === 0 || state.status !== 'connected') {
    return Promise.resolve();
  }
  const payload = bytes instanceof Uint8Array ? new Uint8Array(bytes) : new Uint8Array(bytes);
  const myEpoch = state.epoch;

  const task = state.writeQueue
    .catch(() => {})
    .then(async () => {
      if (state.epoch !== myEpoch || state.status !== 'connected') return;
      if (state.transport === 'webserial' && state.webSerialWriter) {
        await state.webSerialWriter.write(payload);
      } else if (state.transport === 'webusb' && state.webUsbDevice && state.webUsbSession) {
        const ep = state.webUsbPorts[state.webUsbPortIdx]?.outEndpointNumber;
        if (ep != null) {
          const MAX_CHUNK = 2048;
          for (let off = 0; off < payload.byteLength; off += MAX_CHUNK) {
            const slice = payload.slice(off, Math.min(payload.byteLength, off + MAX_CHUNK));
            let res = await state.webUsbDevice.transferOut(ep, slice);
            if (res.status === 'stall') {
              try { await state.webUsbDevice.clearHalt('out', ep); } catch {}
              await state.webUsbDevice.transferOut(ep, slice);
            }
          }
        }
      }
    });

  state.writeQueue = task.catch(() => {});
  return task;
}

export async function js_set_signals(dtr, rts) {
  state.dtr = Boolean(dtr);
  state.rts = Boolean(rts);
  if (state.transport === 'webserial' && state.webSerialPort) {
    await state.webSerialPort.setSignals({ dataTerminalReady: state.dtr, requestToSend: state.rts });
    notifyCurrentState();
  } else if (state.transport === 'webusb' && state.webUsbSession) {
    await state.webUsbSession.setControlLines(state.dtr, state.rts);
    notifyCurrentState();
  }
}

export async function js_pulse_reset() {
  if (state.transport === 'none') return;
  const prevDtr = state.dtr;
  const prevRts = state.rts;
  await js_set_signals(false, true);
  await new Promise((r) => setTimeout(r, 100));
  await js_set_signals(prevDtr, prevRts);
}

export async function js_reconfigure_serial(cfgJson) {
  const parsed = parseConfigJson(cfgJson);
  state.cfg = { ...state.cfg, ...parsed };
  if (state.transport === 'webserial' && state.webSerialPort) {
    await openWebSerial(state.webSerialPort, state.cfg);
  } else if (state.transport === 'webusb' && state.webUsbSession) {
    await state.webUsbSession.setParameters(state.cfg);
  }
}

export async function js_switch_usb_port(portIdx) {
  if (state.transport === 'webusb' && state.webUsbDevice) {
    await openWebUsb(state.webUsbDevice, state.cfg, portIdx);
  }
}

export function js_set_demo_active(active) {
  const next = Boolean(active);
  if (next === state.demoActive) return;
  state.demoActive = next;
  if (!next) return;

  // Demo is a Rust-side simulator. Close any real USB session and suppress
  // auto-reconnect so demo and physical bytes can never be interleaved.
  state.manualDisconnect = true;
  if (state.reconnectTimer) {
    clearTimeout(state.reconnectTimer);
    state.reconnectTimer = null;
  }
  state.epoch += 1;
  notifyState('waiting', 'none', '', '', 0, 0, false, false, '');
  void closeAll(false);
}

export function js_set_auto_connect(enabled, cfgJson) {
  state.autoConnectEnabled = Boolean(enabled);
  if (cfgJson) {
    const parsed = parseConfigJson(cfgJson);
    state.cfg = { ...state.cfg, ...parsed };
  }
  if (state.autoConnectEnabled && !state.manualDisconnect && !state.flashing && !state.demoActive && !state.webSerialPort && !state.webUsbDevice) {
    scheduleAutoReconnect(null, 80);
  } else if (!state.autoConnectEnabled && state.reconnectTimer) {
    clearTimeout(state.reconnectTimer);
    state.reconnectTimer = null;
  }
}

// --- Unified esptool-js Web Flasher ---
export async function js_flash_firmware(
  profileJson,
  forceOtherPort,
  flashOffset,
  customFile,
  bootloaderCmd,
  initCmds,
  onLog,
  onProgress,
) {
  if (typeof navigator === 'undefined' || (!navigator.serial && !navigator.usb)) {
    throw new Error('Web Serial or WebUSB API is required for browser firmware flashing.');
  }

  const profile = JSON.parse(profileJson);
  const baseUrl = (profile.base_url || './firmware').replace(/\/+$/, '');
  if (initCmds) {
    state.initCmds = new Uint8Array(initCmds);
  }

  state.flashing = true;
  if (state.reconnectTimer) {
    clearTimeout(state.reconnectTimer);
    state.reconnectTimer = null;
  }

  const backend = preferredBackend();
  const useWebUsb =
    state.transport === 'webusb' ||
    (state.transport !== 'webserial' && backend === 'webusb');

  let port = null;
  let webUsbDevice = null;
  let savedUsbVid = 0;
  let savedUsbPid = 0;
  let savedUsbSerial = '';
  let flashFailed = false;

  try {
    onLog(`[1/4] Preparing to flash ${profile.label}...`);
    if (useWebUsb && navigator.usb) {
      const activeUsb = forceOtherPort ? null : state.webUsbDevice;
      if (!forceOtherPort && state.transport === 'webusb' && activeUsb) {
        savedUsbVid = activeUsb.vendorId ?? 0;
        savedUsbPid = activeUsb.productId ?? 0;
        savedUsbSerial = activeUsb.serialNumber ?? '';
        const isNativeHwCdc = savedUsbVid === 0x303a && savedUsbPid === 0x1001;
        if (isNativeHwCdc) {
          // On native ESP32-S3/C6 USB-Serial-JTAG, avoid 0xF0 esp_restart() over Android WebUSB
          // because re-enumeration detaches the active USBDevice handle; let esptool-js perform
          // hardware UsbJtagSerialReset directly over the open CDC control lines instead.
          onLog('Closing app WebUSB session for native USB-Serial-JTAG flashing...');
          state.epoch += 1;
          await closeAll(true);
          await new Promise((r) => setTimeout(r, 150));
          webUsbDevice = activeUsb;
        } else {
          onLog('Sending 0xF0 bootloader entry command & waiting for USB re-enumeration...');
          try { await js_write_bytes(bootloaderCmd); } catch {}
          await new Promise((r) => setTimeout(r, 200));
          state.epoch += 1;
          await closeAll(true);
          await new Promise((r) => setTimeout(r, 450));
          webUsbDevice = (await waitForFreshUsbDevice(savedUsbVid, savedUsbPid, savedUsbSerial, 3000)) || activeUsb;
        }
      } else {
        if (!forceOtherPort) {
          // Reuse an already-authorized Espressif/Seeed device (e.g. right after mismatch auto-disconnect)
          try {
            const existing = await navigator.usb.getDevices();
            const esp = existing.find((d) => d.vendorId === 0x303a || d.vendorId === 0x2886);
            if (esp) {
              webUsbDevice = esp;
              onLog(`Using authorized USB device (${formatVidPid(esp.vendorId, esp.productId)})...`);
            }
          } catch {}
        }
        if (!webUsbDevice) {
          onLog('Select the XIAO ESP32 USB-JTAG / CDC device in the browser WebUSB prompt...');
          webUsbDevice = await navigator.usb.requestDevice({ filters: WEBUSB_FILTERS });
        }
        savedUsbVid = webUsbDevice.vendorId ?? 0;
        savedUsbPid = webUsbDevice.productId ?? 0;
        savedUsbSerial = webUsbDevice.serialNumber ?? '';
      }
      port = new WebUsbCdcFlashPort(webUsbDevice);
    } else if (navigator.serial) {
      let serialPort = forceOtherPort ? null : state.webSerialPort;
      if (!forceOtherPort && state.transport === 'webserial' && serialPort) {
        onLog('Sending 0xF0 bootloader entry command & closing serial transport...');
        try { await js_write_bytes(bootloaderCmd); } catch {}
        await new Promise((r) => setTimeout(r, 250));
        state.epoch += 1;
        await closeAll(true);
      } else {
        onLog('Select the XIAO ESP32 USB-JTAG / CDC COM port in the browser prompt...');
        serialPort = await navigator.serial.requestPort();
      }
      port = serialPort;
    }

    if (!port) throw new Error('No serial or USB port selected.');

    const fileArray = [];
    if (customFile) {
      onLog(`Reading selected file: ${customFile.name}...`);
      const buf = new Uint8Array(await customFile.arrayBuffer());
      const addr = flashOffset === '0x0' ? 0x0 : 0x10000;
      fileArray.push({ data: buf, address: addr });
      onLog(`Loaded ${customFile.name} (${buf.byteLength} bytes) -> 0x${addr.toString(16)}`);
    } else if (flashOffset === 'full') {
      onLog('Downloading bundled full firmware images (bootloader + partitions + app0 + app)...');
      const items = [
        { label: 'bootloader', address: profile.bootloader_offset ?? 0x0, url: `${baseUrl}/${profile.prefix}.bootloader.bin` },
        { label: 'partitions', address: 0x8000, url: `${baseUrl}/${profile.prefix}.partitions.bin` },
        { label: 'boot_app0', address: 0xe000, url: `${baseUrl}/boot_app0.bin` },
        { label: 'firmware', address: 0x10000, url: `${baseUrl}/${profile.prefix}.bin` },
      ];
      for (const item of items) {
        const resp = await fetch(item.url);
        if (!resp.ok) throw new Error(`Missing binary ${item.url} (${resp.status}).`);
        const buf = new Uint8Array(await resp.arrayBuffer());
        onLog(`Loaded ${item.label} @ 0x${item.address.toString(16)} (${buf.byteLength} bytes)`);
        fileArray.push({ data: buf, address: item.address });
      }
    } else {
      const appUrl = `${baseUrl}/${profile.prefix}.bin`;
      onLog(`Downloading bundled app firmware (${profile.prefix}.bin)...`);
      const resp = await fetch(appUrl);
      if (!resp.ok) throw new Error(`Missing binary ${appUrl} (${resp.status}).`);
      const buf = new Uint8Array(await resp.arrayBuffer());
      const addr = flashOffset === '0x0' ? 0x0 : 0x10000;
      fileArray.push({ data: buf, address: addr });
      onLog(`Loaded ${profile.prefix}.bin (${buf.byteLength} bytes) -> 0x${addr.toString(16)}`);
    }

    onLog('[2/4] Connecting to ROM bootloader...');
    const { ESPLoader, Transport } = await import('https://esm.sh/esptool-js@0.7.1');
    let espTransport = new Transport(port, false);
    const makeLoader = (tr) =>
      new ESPLoader({
        transport: tr,
        baudrate: 460800,
        romBaudrate: 115200,
        terminal: {
          clean: () => {},
          writeLine: (line) => onLog(line),
          write: (data) => {
            const cleaned = data.replace(/\r/g, '').trim();
            if (cleaned) onLog(cleaned);
          },
        },
      });
    let esploader = makeLoader(espTransport);

    let chip = '';
    try {
      chip = await esploader.main('default_reset');
    } catch {
      onLog('Default reset sync timed out; retrying direct no_reset sync...');
      try { await espTransport.disconnect(); } catch {}
      await new Promise((r) => setTimeout(r, 300));
      if (useWebUsb && navigator.usb) {
        const fresh = await waitForFreshUsbDevice(savedUsbVid, savedUsbPid, savedUsbSerial, 2000);
        if (fresh) {
          webUsbDevice = fresh;
          port = new WebUsbCdcFlashPort(webUsbDevice);
          espTransport = new Transport(port, false);
          esploader = makeLoader(espTransport);
        }
      }
      chip = await esploader.main('no_reset');
    }
    onLog(`Connected to bootloader chip: ${chip || profile.label}`);

    onLog('[3/4] Writing flash memory...');
    const totalBytes = fileArray.reduce((acc, f) => acc + f.data.byteLength, 0);

    // Always pass 'keep' for flashSize / flashMode / flashFreq:
    // 1. All bundled .bootloader.bin images already carry exact Arduino-compiled headers
    //    (ESP32-S3: 0x3f = 8MB/80MHz DIO, ESP32-C6: 0x20 = 4MB/80MHz DIO) and valid SHA-256 digests.
    // 2. esptool-js@0.7.1 has a known bug where patching bootloader headers overwrites C6's
    //    0x20 clock nibble with invalid 0x2f AND fails to recompute the appended SHA-256 digest.
    await esploader.writeFlash({
      fileArray,
      flashSize: 'keep',
      flashMode: 'keep',
      flashFreq: 'keep',
      eraseAll: false,
      compress: true,
      reportProgress: (fileIndex, written, total) => {
        const prev = fileArray.slice(0, fileIndex).reduce((acc, f) => acc + f.data.byteLength, 0);
        const cur = fileArray[fileIndex]
          ? fileArray[fileIndex].data.byteLength * (written / Math.max(1, total))
          : written;
        const ratio = totalBytes > 0 ? (prev + cur) / totalBytes : written / Math.max(1, total);
        onProgress(Math.min(100, Math.round(ratio * 100)));
      },
    });

    onLog('[4/4] Flashing complete! Triggering reset...');
    const isEsp32S3 =
      profile.board_id === 'esp32s3' ||
      (typeof chip === 'string' && chip.toUpperCase().includes('S3'));
    let watchdogTriggered = false;
    if (isEsp32S3) {
      // Clear RTC_CNTL_OPTION1_REG (FORCE_DOWNLOAD_BOOT) before resetting.
      try {
        await esploader.writeReg(0x6000812c, 0);
      } catch {}
      // Official Espressif esptool ESP32-S3 watchdog_reset():
      // WDT_EN (1<<31) | WDT_STG0=5 (5<<28) | WDT_CHIP_RESET_EN (1<<8) | WIDTH=2 = 0xD0000102.
      // Performs a true whole-chip RTC reset that re-samples GPIO0 strapping pins so the
      // ESP32-S3 reliably exits ROM download mode into the newly flashed firmware.
      // 8000 RTC_SLOW_CLK ticks (~60 ms) lets the lock write + transport disconnect finish cleanly.
      try {
        await esploader.writeReg(0x600080b0, 0x50d83aa1); // Unlock RTC WDT
        await esploader.writeReg(0x6000809c, 8000);       // ~60 ms timeout
        await esploader.writeReg(0x60008098, 0xd0000102); // Enable whole-chip RTC WDT reset
        try { await esploader.writeReg(0x600080b0, 0); } catch {} // Re-lock RTC WDT
        watchdogTriggered = true;
        onLog('ESP32-S3 whole-chip watchdog reset armed.');
      } catch {}
    }
    if (!watchdogTriggered) {
      // Explicit USB-Serial-JTAG / UART hardware reset pulse (with DTR=false settled first!).
      // Note: esptool-js@0.7.1 after('hard_reset') omits asserting RTS=true before RTS=false,
      // and ESP32-C6 forbids RTC WDT reset, so an explicit DTR=0 -> RTS=1 -> RTS=0 pulse is required.
      try {
        await espTransport.setDTR(false);
        await espTransport.setRTS(false);
        await new Promise((r) => setTimeout(r, 50));
        await espTransport.setRTS(true);
        await new Promise((r) => setTimeout(r, 150));
        await espTransport.setRTS(false);
        await new Promise((r) => setTimeout(r, 150));
        onLog('Hardware RTS reset pulse sent.');
      } catch {}
    }
    const serialInfo = !webUsbDevice && port?.getInfo ? port.getInfo() : {};
    try { await espTransport.disconnect(); } catch {}

    onProgress(100);
    onLog('Flash complete! Waiting for board reboot & reconnecting...');
    await new Promise((r) => setTimeout(r, watchdogTriggered ? 1600 : 1200));
    state.flashing = false;
    state.manualDisconnect = false;
    if (webUsbDevice) {
      // ESP32-S3 whole-chip watchdog reset re-enumerates USB on Android, invalidating old USBDevice handles.
      const fresh =
        (await waitForFreshUsbDevice(
          savedUsbVid,
          savedUsbPid,
          savedUsbSerial,
          4000,
          watchdogTriggered ? webUsbDevice : null,
          1000,
        )) || webUsbDevice;
      const ok = await openWebUsb(fresh, state.cfg, 0);
      if (!ok) {
        scheduleAutoReconnect(null, 900);
      }
    } else {
      const targetPort =
        (watchdogTriggered
          ? await waitForFreshSerialPort(serialInfo.usbVendorId ?? 0, serialInfo.usbProductId ?? 0, 3000)
          : null) || port;
      const ok = await openWebSerial(targetPort, state.cfg);
      if (!ok) {
        scheduleAutoReconnect(null, 900);
      }
    }
  } catch (err) {
    flashFailed = true;
    // A failed flash can leave the app transport closed (we closed it before
    // invoking esptool-js) while the UI still believes it is connected. Tear
    // down both the temporary flasher port and any stale transport state.
    try {
      if (port && typeof port.close === 'function') await port.close();
    } catch {}
    try {
      if (webUsbDevice && webUsbDevice.opened) await webUsbDevice.close();
    } catch {}
    state.epoch += 1;
    await closeAll(false);
    const msg = err instanceof Error ? err.message : String(err);
    notifyState('waiting', 'none', '', '', 0, 0, false, false, msg || 'Firmware flashing failed');
    throw err;
  } finally {
    state.flashing = false;
    if (flashFailed) {
      scheduleAutoReconnect(null, 900);
    }
  }
}
