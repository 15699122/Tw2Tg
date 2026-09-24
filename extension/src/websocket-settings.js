export const WEBSOCKET_DEFAULT_PORT = 17321;
export const WEBSOCKET_SETTINGS_KEY = "xarchiveWebSocketSettings";
export const DEFAULT_WEBSOCKET_SETTINGS = Object.freeze({ enabled: false, port: WEBSOCKET_DEFAULT_PORT, token: "" });

export function normalizeWebSocketSettings(value = {}) {
  const port = Number(value.port);
  return {
    enabled: value.enabled === true,
    port: Number.isInteger(port) && port > 0 && port <= 65535 ? port : WEBSOCKET_DEFAULT_PORT,
    token: typeof value.token === "string" ? value.token.trim() : "",
  };
}

export function readWebSocketSettings(api) {
  return new Promise((resolve) => {
    if (!api?.storage?.local?.get) return resolve({ ...DEFAULT_WEBSOCKET_SETTINGS });
    api.storage.local.get(WEBSOCKET_SETTINGS_KEY, (result) => resolve(normalizeWebSocketSettings(result?.[WEBSOCKET_SETTINGS_KEY])));
  });
}

export function writeWebSocketSettings(api, value) {
  const settings = normalizeWebSocketSettings(value);
  return new Promise((resolve, reject) => {
    if (!api?.storage?.local?.set) return resolve(settings);
    api.storage.local.set({ [WEBSOCKET_SETTINGS_KEY]: settings }, () => {
      const error = api.runtime?.lastError;
      if (error) reject(new Error(error.message));
      else resolve(settings);
    });
  });
}
