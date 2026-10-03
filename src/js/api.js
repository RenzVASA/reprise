// Pont vers le cœur Rust. Hors de l'app (ouvert dans un navigateur pour bricoler
// le design), on bascule sur des données de démonstration.
import { demo } from "./demo.js";

const T = window.__TAURI__;
export const inApp = Boolean(T && T.core);

export function call(cmd, args = {}) {
  return inApp ? T.core.invoke(cmd, args) : demo.invoke(cmd, args);
}

export function on(event, handler) {
  return inApp ? T.event.listen(event, (e) => handler(e.payload)) : demo.listen(event, handler);
}

// Raccourcis lisibles des commandes.
export const api = {
  list: () => call("list_contexts"),
  update: (id, patch) => call("update_context", { id, patch }),
  remove: (id) => call("delete_context", { id }),
  putBack: (context) => call("put_back_context", { context }),
  startCapture: (replace = null) => call("start_capture", { replace }),
  pending: () => call("get_pending_capture"),
  saveCapture: (name, note, exclude = null) => call("save_capture", { name, note, exclude }),
  cancelCapture: () => call("cancel_capture"),
  restore: (id, items = null, tidy = null) => call("restore_context", { id, items, tidy }),
  openItem: (id, item) => call("open_item", { id, item }),
  note: () => call("get_note"),
  closeNote: () => call("close_note"),
  openMain: () => call("open_main"),
  settings: () => call("get_settings"),
  saveSettings: (settings) => call("set_settings", { settings }),
  pauseShortcuts: (paused) => call("pause_shortcuts", { paused }),
  permissions: () => call("check_permissions"),
  openPrivacy: (kind) => call("open_privacy_pane", { kind }),
  revealData: () => call("reveal_data"),
  openLink: (url) => call("open_link", { url }),
  info: () => call("app_info"),
  ignoreApp: (bundleId, name) => call("ignore_app", { bundleId, name }),
  checkUpdates: () => call("check_updates"),
  getUpdate: () => call("get_update"),
};
