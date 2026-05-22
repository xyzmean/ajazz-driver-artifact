<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import {
  listDevices,
  getDeviceInfo,
  openSettingsWindow,
  getGameMode,
  setGameMode,
  getLedEffect,
  setLedEffect,
  getMagneticRt,
  setMagneticRt,
  factoryReset,
  type DeviceSummary,
  type DeviceInfo,
  type GameMode,
  type LedEffect,
  type MagneticAxisRT
} from "./lib/api";

// URL Query Params routing
const params = new URLSearchParams(window.location.search);
const isSettingsWindow = ref(params.get("window") === "settings");

// Decode hex utility
function decodeHex(hex: string): string {
  if (!hex) return "";
  let str = "";
  for (let i = 0; i < hex.length; i += 2) {
    str += String.fromCharCode(parseInt(hex.substring(i, i + 2), 16));
  }
  return str;
}

// Device context for settings window
const rawPath = params.get("path") || "";
const rawModel = params.get("model") || "";
const settingsPath = decodeHex(rawPath);
const settingsModel = decodeHex(rawModel);

// Shared States
const devices = ref<DeviceSummary[]>([]);
const selected = ref<DeviceSummary | null>(null);
const info = ref<DeviceInfo | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const successMessage = ref<string | null>(null);

// Settings Tab System
const activeTab = ref<"rgb" | "rt" | "system">("rgb");

// Configurations States
const rgbEffect = ref<LedEffect>({
  mode: 0,
  red: 255,
  green: 0,
  blue: 255,
  driverSetting: 255,
  secondaryRed: 0,
  secondaryGreen: 0,
  secondaryBlue: 0,
  colorMode: 0,
  brightness: 4,
  speed: 3,
  direction: 0,
  effectModeType: 0
});

const systemConfig = ref<GameMode>({
  gameMode: 0,
  fnSwitch: 0,
  sleepTime: 5,
  keyDelay: 0,
  reportRate: 4,
  systemMode: 0,
  tftDisplayTime: 10,
  topDeadZone: 0.05,
  bottomDeadZone: 0.05,
  stabilityMode: 0,
  autoCalibration: 1,
  singleKeyWakeup: 1
});

const rtGlobal = ref({
  isWholeFast: true,
  isRampageMode: false,
  triggerKeyStroke: 1.5,
  pressRt: 0.5,
  releaseRt: 0.5
});

// Hex colors utility
const hexColor = ref("#ff00ff");
watch(hexColor, (newVal) => {
  const r = parseInt(newVal.slice(1, 3), 16);
  const g = parseInt(newVal.slice(3, 5), 16);
  const b = parseInt(newVal.slice(5, 7), 16);
  rgbEffect.value.red = r;
  rgbEffect.value.green = g;
  rgbEffect.value.blue = b;
});

function applyHexPreset(preset: string) {
  hexColor.value = preset;
}

// Polling Rates list
const pollingRates = [
  { value: 1, label: "125 Hz" },
  { value: 2, label: "250 Hz" },
  { value: 3, label: "500 Hz" },
  { value: 4, label: "1000 Hz" }
];

// RGB effects list
const rgbModes = [
  { value: 0, label: "Статический" },
  { value: 1, label: "Волна (Влево)" },
  { value: 2, label: "Волна (Вправо)" },
  { value: 3, label: "Дыхание" },
  { value: 4, label: "Неон" },
  { value: 5, label: "Рябь" },
  { value: 6, label: "Светящиеся клавиши" },
  { value: 7, label: "Спектр" },
  { value: 8, label: "Случайный всплеск" },
  { value: 9, label: "Радар" }
];

// Helper to show temporary success message
function notifySuccess(msg: string) {
  successMessage.value = msg;
  setTimeout(() => {
    successMessage.value = null;
  }, 3000);
}

// Primary actions
async function refreshDevices() {
  error.value = null;
  loading.value = true;
  try {
    devices.value = await listDevices();
    if (devices.value.length && !selected.value) {
      await selectDevice(devices.value[0]);
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function selectDevice(dev: DeviceSummary) {
  selected.value = dev;
  error.value = null;
  info.value = null;
  loading.value = true;
  try {
    info.value = await getDeviceInfo(dev.path);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

// Open settings window from main dashboard
async function openSettings() {
  if (!selected.value) return;
  try {
    await openSettingsWindow(selected.value.path, selected.value.modelName ?? selected.value.product ?? "Keyboard");
  } catch (e) {
    error.value = `Не удалось открыть настройки: ${e}`;
  }
}

// Load configurations in settings window
async function loadSettingsData() {
  if (!settingsPath) return;
  loading.value = true;
  error.value = null;
  try {
    // 1. Get basic DeviceInfo
    const devInfo = await getDeviceInfo(settingsPath);
    info.value = devInfo;

    // 2. Fetch specific panels in parallel
    const [led, game, rt] = await Promise.all([
      getLedEffect(settingsPath, devInfo.frameVersion).catch(() => null),
      getGameMode(settingsPath, devInfo.frameVersion).catch(() => null),
      getMagneticRt(settingsPath, devInfo.rtPrecision, devInfo.frameVersion).catch(() => null)
    ]);

    if (led) {
      rgbEffect.value = led;
      const rHex = led.red.toString(16).padStart(2, "0");
      const gHex = led.green.toString(16).padStart(2, "0");
      const bHex = led.blue.toString(16).padStart(2, "0");
      hexColor.value = `#${rHex}${gHex}${bHex}`;
    }

    if (game) {
      systemConfig.value = game;
    }

    if (rt && rt.length > 0) {
      // Pick values from first key for global slides
      const first = rt[0];
      rtGlobal.value = {
        isWholeFast: first.isWholeFast,
        isRampageMode: first.isRampageMode,
        triggerKeyStroke: first.triggerKeyStroke,
        pressRt: first.pressRt,
        releaseRt: first.releaseRt
      };
    }
  } catch (e) {
    error.value = `Ошибка загрузки настроек: ${e}`;
  } finally {
    loading.value = false;
  }
}

// Apply settings
async function saveRgb() {
  if (!settingsPath) return;
  loading.value = true;
  error.value = null;
  try {
    await setLedEffect(settingsPath, rgbEffect.value);
    notifySuccess("Параметры RGB успешно применены!");
  } catch (e) {
    error.value = `Ошибка сохранения RGB: ${e}`;
  } finally {
    loading.value = false;
  }
}

async function saveSystem() {
  if (!settingsPath || !info.value) return;
  loading.value = true;
  error.value = null;
  try {
    await setGameMode(settingsPath, systemConfig.value, info.value.frameVersion);
    notifySuccess("Системные настройки успешно применены!");
  } catch (e) {
    error.value = `Ошибка сохранения настроек: ${e}`;
  } finally {
    loading.value = false;
  }
}

async function saveRt() {
  if (!settingsPath || !info.value) return;
  loading.value = true;
  error.value = null;
  try {
    // Reconstruct 128 keys from global states
    const keys: MagneticAxisRT[] = Array.from({ length: 128 }, () => ({
      axisType: 1, // standard magnetic axis type
      isWholeFast: rtGlobal.value.isWholeFast,
      isRampageMode: rtGlobal.value.isRampageMode,
      triggerKeyStroke: rtGlobal.value.triggerKeyStroke,
      pressRt: rtGlobal.value.pressRt,
      releaseRt: rtGlobal.value.releaseRt
    }));

    await setMagneticRt(settingsPath, info.value.rtPrecision, keys, info.value.frameVersion);
    notifySuccess("Параметры Rapid Trigger успешно применены!");
  } catch (e) {
    error.value = `Ошибка сохранения Rapid Trigger: ${e}`;
  } finally {
    loading.value = false;
  }
}

async function triggerReset(type: number) {
  if (!settingsPath) return;
  const label = type === 255 ? "Все параметры" : "Выбранные настройки";
  if (!confirm(`Вы действительно хотите сбросить клавиатуру к заводским настройкам?\n(${label})`)) {
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    await factoryReset(settingsPath, type);
    notifySuccess("Настройки клавиатуры сброшены!");
    await loadSettingsData();
  } catch (e) {
    error.value = `Не удалось сбросить настройки: ${e}`;
  } finally {
    loading.value = false;
  }
}

const hex = (n: number) => "0x" + n.toString(16).padStart(4, "0");

onMounted(async () => {
  if (isSettingsWindow.value) {
    await loadSettingsData();
  } else {
    await refreshDevices();
  }
});
</script>

<template>
  <!-- 1. MAIN WINDOW INTERFACE -->
  <div v-if="!isSettingsWindow" class="app glass-bg">
    <header class="topbar">
      <div class="logo">
        <div class="glow-dot"></div>
        <h1>AJAZZ <span>DRIVER</span></h1>
      </div>
      <button class="btn btn-primary" :disabled="loading" @click="refreshDevices">
        <span v-if="loading" class="spinner"></span>
        <span>{{ loading ? "Сканирование..." : "Обновить" }}</span>
      </button>
    </header>

    <main class="layout">
      <!-- Device list sidebar -->
      <section class="sidebar glass-panel">
        <h2 class="section-title">Подключенные устройства</h2>
        <p v-if="!devices.length && !loading" class="muted-text">
          Клавиатуры не найдены. Подключите устройство по USB и нажмите кнопку «Обновить».
        </p>
        <ul class="device-list">
          <li
            v-for="d in devices"
            :key="d.path"
            :class="{ active: selected?.path === d.path }"
            @click="selectDevice(d)"
            class="device-item"
          >
            <div class="device-glow"></div>
            <div class="device-icon">⌨</div>
            <div class="device-info-wrapper">
              <span class="device-name">{{ d.modelName ?? d.product ?? "AJAZZ Keyboard" }}</span>
              <span class="device-ids">{{ hex(d.vendorId) }}:{{ hex(d.productId) }}</span>
            </div>
          </li>
        </ul>
      </section>

      <!-- Central details view -->
      <section class="detail glass-panel">
        <div v-if="error" class="error-banner">{{ error }}</div>
        
        <template v-if="info">
          <div class="detail-header">
            <h2>{{ selected?.modelName ?? selected?.product ?? "Игровая клавиатура" }}</h2>
            <div class="badge">USB Подключение</div>
          </div>

          <!-- Futuristic battery charge display -->
          <div class="battery-section">
            <div class="battery-label">
              <span>Заряд батареи</span>
              <span class="battery-percent">{{ info.batteryLevel }}%</span>
            </div>
            <div class="battery-track">
              <div 
                class="battery-fill" 
                :style="{ width: `${info.batteryLevel}%` }"
                :class="{ 'battery-charging': info.chargeStatus === 1 }"
              >
                <div class="liquid-wave"></div>
              </div>
            </div>
            <div class="battery-status-text" v-if="info.chargeStatus === 1">
              ⚡ Устройство заряжается
            </div>
          </div>

          <!-- Specs grid -->
          <div class="info-grid">
            <div class="info-card">
              <label>Версия прошивки</label>
              <b>v{{ info.version.toFixed(2) }}</b>
            </div>
            <div class="info-card">
              <label>Идентификатор (VID)</label>
              <b>{{ hex(info.vendorId) }}</b>
            </div>
            <div class="info-card">
              <label>Идентификатор (PID)</label>
              <b>{{ hex(info.productId) }}</b>
            </div>
            <div class="info-card">
              <label>Активный профиль</label>
              <b>#{{ info.currentProfile }}</b>
            </div>
            <div class="info-card">
              <label>Режим работы</label>
              <b>{{ info.workMode === 1 ? "Беспроводной" : "Кабель" }}</b>
            </div>
            <div class="info-card">
              <label>Точность RT</label>
              <b>{{ info.rtPrecision }}</b>
            </div>
          </div>

          <!-- Configuration Entry Button -->
          <div class="action-footer">
            <button class="btn btn-glow" @click="openSettings">
              <span>⚙ Настроить устройство</span>
            </button>
          </div>
        </template>
        
        <div v-else-if="!error && !loading" class="empty-state">
          <div class="pulse-ring"></div>
          <p class="muted-text">Выберите устройство из списка слева для просмотра информации</p>
        </div>
      </section>
    </main>
  </div>

  <!-- 2. SETTINGS WINDOW INTERFACE -->
  <div v-else class="settings-window glass-bg">
    <div class="settings-header">
      <div class="settings-title">
        <span class="glow-dot"></span>
        <h2>Центр настройки <span>{{ settingsModel }}</span></h2>
      </div>
      <div class="settings-nav">
        <button 
          :class="{ active: activeTab === 'rgb' }" 
          @click="activeTab = 'rgb'" 
          class="nav-tab"
        >
          🎨 Подсветка RGB
        </button>
        <button 
          :class="{ active: activeTab === 'rt' }" 
          @click="activeTab = 'rt'" 
          class="nav-tab"
        >
          ⚡ Rapid Trigger
        </button>
        <button 
          :class="{ active: activeTab === 'system' }" 
          @click="activeTab = 'system'" 
          class="nav-tab"
        >
          ⚙ Система
        </button>
      </div>
    </div>

    <div v-if="error" class="error-banner">{{ error }}</div>
    <div v-if="successMessage" class="success-banner">{{ successMessage }}</div>

    <div class="settings-body glass-panel">
      <div v-if="loading && !error" class="settings-loading">
        <div class="spinner-large"></div>
        <p>Запись и применение данных на клавиатуру...</p>
      </div>

      <!-- Tab: RGB -->
      <div v-else-if="activeTab === 'rgb'" class="tab-content">
        <div class="rgb-layout">
          <!-- Left side controls -->
          <div class="rgb-controls">
            <div class="control-group">
              <label>Световой эффект</label>
              <select v-model="rgbEffect.mode" class="select-input">
                <option v-for="mode in rgbModes" :key="mode.value" :value="mode.value">
                  {{ mode.label }}
                </option>
              </select>
            </div>

            <div class="control-group">
              <div class="slider-header">
                <label>Яркость подсветки</label>
                <span class="value-badge">{{ Math.round((rgbEffect.brightness / 4) * 100) }}%</span>
              </div>
              <input 
                type="range" 
                min="0" 
                max="4" 
                v-model.number="rgbEffect.brightness" 
                class="range-slider"
              />
              <div class="slider-labels">
                <span>Выкл</span>
                <span>Макс</span>
              </div>
            </div>

            <div class="control-group">
              <div class="slider-header">
                <label>Скорость анимации</label>
                <span class="value-badge">{{ rgbEffect.speed }}/5</span>
              </div>
              <input 
                type="range" 
                min="1" 
                max="5" 
                v-model.number="rgbEffect.speed" 
                class="range-slider"
              />
              <div class="slider-labels">
                <span>Медленно</span>
                <span>Быстро</span>
              </div>
            </div>

            <div class="control-group">
              <label>Направление движения</label>
              <div class="btn-group">
                <button 
                  :class="{ active: rgbEffect.direction === 0 }" 
                  @click="rgbEffect.direction = 0" 
                  class="btn btn-outline btn-small"
                >
                  ⬅ Влево / Назад
                </button>
                <button 
                  :class="{ active: rgbEffect.direction === 1 }" 
                  @click="rgbEffect.direction = 1" 
                  class="btn btn-outline btn-small"
                >
                  ➡ Вправо / Вперед
                </button>
              </div>
            </div>
          </div>

          <!-- Right side colorpicker -->
          <div class="rgb-color-panel">
            <div class="color-picker-box">
              <label>Основной цвет</label>
              <div class="picker-row">
                <input type="color" v-model="hexColor" class="custom-color-picker" />
                <span class="hex-text">{{ hexColor.toUpperCase() }}</span>
              </div>
            </div>

            <div class="presets-box">
              <label>Неоновые пресеты</label>
              <div class="color-presets">
                <button 
                  v-for="color in ['#ff00ff', '#00ffff', '#ffff00', '#9d00ff', '#00ff00', '#ff0000', '#ffffff']" 
                  :key="color"
                  class="preset-dot"
                  :style="{ backgroundColor: color }"
                  :class="{ active: hexColor === color }"
                  @click="applyHexPreset(color)"
                ></button>
              </div>
            </div>

            <div class="rgb-preview" :style="{ '--glow-color': hexColor }">
              <div class="keyboard-preview-glow"></div>
              <div class="preview-keyboard-mock">
                <span>⌨</span>
                <p>Цветовая заливка клавиш</p>
              </div>
            </div>
          </div>
        </div>

        <div class="settings-footer">
          <button class="btn btn-glow" @click="saveRgb">Сохранить подсветку</button>
        </div>
      </div>

      <!-- Tab: Rapid Trigger -->
      <div v-else-if="activeTab === 'rt'" class="tab-content">
        <div class="rt-layout">
          <!-- Controls -->
          <div class="rt-controls">
            <div class="rt-header-row">
              <h3>Магнитные переключатели</h3>
              <div class="toggles">
                <label class="toggle-container">
                  <input type="checkbox" v-model="rtGlobal.isWholeFast" />
                  <span class="toggle-label">Ускоренный триггер (Fast)</span>
                </label>
                <label class="toggle-container">
                  <input type="checkbox" v-model="rtGlobal.isRampageMode" />
                  <span class="toggle-label">Режим Rampage</span>
                </label>
              </div>
            </div>

            <div class="control-group">
              <div class="slider-header">
                <label>Точка срабатывания (Actuation Point)</label>
                <span class="value-badge">{{ rtGlobal.triggerKeyStroke.toFixed(1) }} мм</span>
              </div>
              <input 
                type="range" 
                min="0.1" 
                max="4.0" 
                step="0.1"
                v-model.number="rtGlobal.triggerKeyStroke" 
                class="range-slider"
              />
              <div class="slider-labels">
                <span>0.1 мм (Мгновенно)</span>
                <span>4.0 мм (Полный ход)</span>
              </div>
            </div>

            <div class="control-group">
              <div class="slider-header">
                <label>Press RT (Чувствительность нажатия)</label>
                <span class="value-badge">{{ rtGlobal.pressRt.toFixed(1) }} мм</span>
              </div>
              <input 
                type="range" 
                min="0.1" 
                max="4.0" 
                step="0.1"
                v-model.number="rtGlobal.pressRt" 
                class="range-slider"
              />
              <div class="slider-labels">
                <span>0.1 мм (Высокая)</span>
                <span>4.0 мм (Низкая)</span>
              </div>
            </div>

            <div class="control-group">
              <div class="slider-header">
                <label>Release RT (Чувствительность отпускания)</label>
                <span class="value-badge">{{ rtGlobal.releaseRt.toFixed(1) }} мм</span>
              </div>
              <input 
                type="range" 
                min="0.1" 
                max="4.0" 
                step="0.1"
                v-model.number="rtGlobal.releaseRt" 
                class="range-slider"
              />
              <div class="slider-labels">
                <span>0.1 мм (Высокая)</span>
                <span>4.0 мм (Низкая)</span>
              </div>
            </div>
          </div>

          <!-- Switch visualizer -->
          <div class="rt-visualizer">
            <div class="switch-box">
              <div class="switch-housing">
                <div 
                  class="switch-stem" 
                  :style="{ transform: `translateY(${ (rtGlobal.triggerKeyStroke / 4) * 40 }px)` }"
                >
                  <div class="switch-magnet" :style="{ backgroundColor: rtGlobal.isRampageMode ? '#ff00ff' : '#00ffff' }"></div>
                </div>
                <div class="switch-contacts"></div>
                <div class="switch-sensor">
                  <div class="magnetic-field" :class="{ active: rtGlobal.isWholeFast }"></div>
                </div>
              </div>
              <span class="switch-caption">Визуализация хода штока ({{ rtGlobal.triggerKeyStroke.toFixed(1) }} мм)</span>
            </div>
          </div>
        </div>

        <div class="settings-footer">
          <button class="btn btn-glow" @click="saveRt">Сохранить Rapid Trigger</button>
        </div>
      </div>

      <!-- Tab: System -->
      <div v-else-if="activeTab === 'system'" class="tab-content">
        <div class="system-layout">
          <!-- Left side -->
          <div class="system-left">
            <div class="control-group">
              <div class="slider-header">
                <label>Верхняя мертвая зона (Top Dead Zone)</label>
                <span class="value-badge">{{ Math.round(systemConfig.topDeadZone * 100) }}%</span>
              </div>
              <input 
                type="range" 
                min="0.0" 
                max="1.0" 
                step="0.01"
                v-model.number="systemConfig.topDeadZone" 
                class="range-slider"
              />
              <div class="slider-labels">
                <span>0%</span>
                <span>100%</span>
              </div>
            </div>

            <div class="control-group">
              <div class="slider-header">
                <label>Нижняя мертвая зона (Bottom Dead Zone)</label>
                <span class="value-badge">{{ Math.round(systemConfig.bottomDeadZone * 100) }}%</span>
              </div>
              <input 
                type="range" 
                min="0.0" 
                max="1.0" 
                step="0.01"
                v-model.number="systemConfig.bottomDeadZone" 
                class="range-slider"
              />
              <div class="slider-labels">
                <span>0%</span>
                <span>100%</span>
              </div>
            </div>

            <div class="control-group">
              <div class="slider-header">
                <label>Таймер автоотключения спящего режима</label>
                <span class="value-badge">{{ systemConfig.sleepTime }} мин</span>
              </div>
              <input 
                type="range" 
                min="1" 
                max="30" 
                v-model.number="systemConfig.sleepTime" 
                class="range-slider"
              />
              <div class="slider-labels">
                <span>1 мин</span>
                <span>30 мин</span>
              </div>
            </div>
          </div>

          <!-- Right side -->
          <div class="system-right">
            <div class="control-group">
              <label>Частота опроса шины USB (Polling Rate)</label>
              <select v-model="systemConfig.reportRate" class="select-input">
                <option v-for="rate in pollingRates" :key="rate.value" :value="rate.value">
                  {{ rate.label }}
                </option>
              </select>
            </div>

            <div class="control-group">
              <label>Системные переключатели</label>
              <div class="toggles-grid">
                <label class="toggle-container">
                  <input type="checkbox" :checked="systemConfig.gameMode === 1" @change="systemConfig.gameMode = systemConfig.gameMode === 1 ? 0 : 1" />
                  <span class="toggle-label">Игровой режим (Win Lock)</span>
                </label>
                <label class="toggle-container">
                  <input type="checkbox" :checked="systemConfig.autoCalibration === 1" @change="systemConfig.autoCalibration = systemConfig.autoCalibration === 1 ? 0 : 1" />
                  <span class="toggle-label">Автокалибровка переключателей</span>
                </label>
                <label class="toggle-container">
                  <input type="checkbox" :checked="systemConfig.singleKeyWakeup === 1" @change="systemConfig.singleKeyWakeup = systemConfig.singleKeyWakeup === 1 ? 0 : 1" />
                  <span class="toggle-label">Пробуждение по любой клавише</span>
                </label>
              </div>
            </div>

            <div class="factory-reset-section glass-panel">
              <h4>🔥 Заводской сброс устройства</h4>
              <p class="muted-text">Вы можете сбросить подсветку, макросы или полностью обнулить энергонезависимую память клавиатуры.</p>
              <div class="reset-buttons">
                <button class="btn btn-outline btn-small" @click="triggerReset(2)">Сбросить RGB</button>
                <button class="btn btn-danger btn-small" @click="triggerReset(255)">Сбросить всё (Reset All)</button>
              </div>
            </div>
          </div>
        </div>

        <div class="settings-footer">
          <button class="btn btn-glow" @click="saveSystem">Сохранить систему</button>
        </div>
      </div>
    </div>
  </div>
</template>
