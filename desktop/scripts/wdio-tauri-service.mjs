import TauriWorkerService, { launcher } from "@wdio/tauri-service";

/**
 * Project WDIO worker adapter.
 *
 * The official service's focus recovery and session teardown assume that
 * tauri-plugin-wdio is present in every native artifact. This project
 * intentionally keeps the plugin out of ordinary release builds, so the
 * adapter retains the official launcher while making worker cleanup safe for
 * both ordinary and wdio-e2e artifacts.
 */
export default class Tw2TgTauriWorkerService extends TauriWorkerService {
  async beforeCommand() {
    // This project has one stable main window. The upstream focus probe calls
    // plugin:wdio|get_window_states before DOM commands, which is unavailable
    // in the ordinary artifact by design.
  }

  async afterSession() {
    const browser = this.browser;
    if (!browser || browser.isMultiremote) {
      return;
    }

    if (browser.sessionId) {
      await browser.deleteSession();
    }
  }
}

export { launcher };