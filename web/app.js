import init, { RustyTrackerWasmEngine } from "./pkg/rustytracker_wasm.js";

const AUDIO_BUFFER_FRAMES = 2048;
const TRACKER_FILE_EXTENSIONS = [".xm", ".mod"];

const elements = {
  fileInput: document.querySelector("#moduleFile"),
  playButton: document.querySelector("#playButton"),
  stopButton: document.querySelector("#stopButton"),
  resetButton: document.querySelector("#resetButton"),
  wasmStatus: document.querySelector("#wasmStatus"),
  fileName: document.querySelector("#fileName"),
  sampleRate: document.querySelector("#sampleRate"),
  playState: document.querySelector("#playState"),
  orderValue: document.querySelector("#orderValue"),
  rowValue: document.querySelector("#rowValue"),
  tickValue: document.querySelector("#tickValue"),
  logOutput: document.querySelector("#logOutput"),
};

const appState = {
  wasmReady: false,
  fileName: "",
  playing: false,
  loaded: false,
  lastError: "",
};

window.__rustytrackerWeb = appState;

let audioContext = null;
let processor = null;
let engine = null;
let loadedBytes = null;
let animationFrame = 0;

boot();

async function boot() {
  setTransportEnabled(false);
  setStatus("WASM loading");

  try {
    await init({ module_or_path: "./pkg/rustytracker_wasm_bg.wasm" });
    appState.wasmReady = true;
    setStatus("WASM ready");
    log("No module loaded.");
  } catch (error) {
    appState.lastError = errorMessage(error);
    setStatus("WASM failed");
    log(`Failed to load WASM: ${appState.lastError}`);
  }
}

elements.fileInput.addEventListener("change", async (event) => {
  const [file] = event.target.files;
  if (!file) {
    return;
  }

  await loadModuleFile(file);
});

elements.playButton.addEventListener("click", async () => {
  if (!engine) {
    return;
  }

  await ensureAudioContext();
  audioContext.resume();
  appState.playing = true;
  elements.playState.textContent = "Playing";
  elements.stopButton.disabled = false;
  elements.playButton.disabled = true;
  log(`Playing ${appState.fileName}`);
});

elements.stopButton.addEventListener("click", () => {
  stopPlayback("Stopped");
});

elements.resetButton.addEventListener("click", async () => {
  if (!loadedBytes || !appState.fileName) {
    return;
  }

  stopPlayback("Reloaded");
  await createEngine(loadedBytes, appState.fileName);
});

async function loadModuleFile(file) {
  if (!appState.wasmReady) {
    log("WASM is not ready yet.");
    return;
  }

  if (!isTrackerFile(file.name)) {
    log(`Unsupported file extension for ${file.name}`);
    return;
  }

  stopPlayback("Loading");
  const bytes = new Uint8Array(await file.arrayBuffer());
  await createEngine(bytes, file.name);
}

async function createEngine(bytes, fileName) {
  try {
    engine = new RustyTrackerWasmEngine(bytes);
    loadedBytes = bytes;
    appState.fileName = fileName;
    appState.loaded = true;
    appState.lastError = "";
    elements.fileName.textContent = fileName;
    elements.playState.textContent = "Loaded";
    setTransportEnabled(true);
    updateCursorReadout();
    log(`Loaded ${fileName} (${bytes.byteLength.toLocaleString()} bytes).`);
  } catch (error) {
    engine = null;
    loadedBytes = null;
    appState.fileName = "";
    appState.loaded = false;
    appState.lastError = errorMessage(error);
    setTransportEnabled(false);
    elements.fileName.textContent = "None loaded";
    elements.playState.textContent = "Load failed";
    updateCursorReadout();
    log(`Could not load ${fileName}: ${appState.lastError}`);
  }
}

async function ensureAudioContext() {
  if (audioContext) {
    return;
  }

  audioContext = new AudioContext();
  elements.sampleRate.textContent = `${Math.round(audioContext.sampleRate)} Hz`;

  processor = audioContext.createScriptProcessor(AUDIO_BUFFER_FRAMES, 0, 2);
  processor.onaudioprocess = (event) => {
    const left = event.outputBuffer.getChannelData(0);
    const right = event.outputBuffer.getChannelData(1);

    if (!appState.playing || !engine) {
      left.fill(0.0);
      right.fill(0.0);
      return;
    }

    engine.render_stereo(Math.round(audioContext.sampleRate), left, right);

    if (engine.song_ended()) {
      appState.playing = false;
      elements.playState.textContent = "Ended";
      elements.playButton.disabled = false;
      elements.stopButton.disabled = true;
    }
  };
  processor.connect(audioContext.destination);

  startCursorLoop();
}

function startCursorLoop() {
  if (animationFrame) {
    return;
  }

  const tick = () => {
    updateCursorReadout();
    animationFrame = requestAnimationFrame(tick);
  };
  animationFrame = requestAnimationFrame(tick);
}

function stopPlayback(label) {
  appState.playing = false;
  elements.playState.textContent = label;
  elements.playButton.disabled = !engine;
  elements.stopButton.disabled = true;
}

function setTransportEnabled(enabled) {
  elements.playButton.disabled = !enabled;
  elements.stopButton.disabled = true;
  elements.resetButton.disabled = !enabled;
}

function updateCursorReadout() {
  if (!engine) {
    elements.orderValue.textContent = "00";
    elements.rowValue.textContent = "00";
    elements.tickValue.textContent = "00";
    return;
  }

  elements.orderValue.textContent = toHex2(engine.current_order());
  elements.rowValue.textContent = toHex2(engine.current_row());
  elements.tickValue.textContent = toHex2(engine.current_tick());
}

function isTrackerFile(fileName) {
  const lowerName = fileName.toLowerCase();
  return TRACKER_FILE_EXTENSIONS.some((extension) => lowerName.endsWith(extension));
}

function setStatus(text) {
  elements.wasmStatus.textContent = text;
}

function log(message) {
  elements.logOutput.textContent = message;
}

function toHex2(value) {
  return Number(value).toString(16).toUpperCase().padStart(2, "0");
}

function errorMessage(error) {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
