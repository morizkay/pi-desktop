import { invoke as tauriInvoke, isTauri } from "@tauri-apps/api/core";
import { validateResult } from "./protocol";

export const hasDesktop = () => typeof window !== "undefined" && isTauri();
async function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!hasDesktop())
    throw new Error(
      "Disconnected preview — open Pi Console in the desktop app to connect.",
    );
  return validateResult(await tauriInvoke<T>(command, args));
}

export type PiSessionSummary = {
  path: string;
  id: string;
  cwd: string;
  title: string;
  updatedAt: string;
};

export type PiSettingsView = {
  defaultProvider?: string;
  defaultModel?: string;
  defaultThinkingLevel?: string;
  packages: string[];
  enabledModels?: string[];
};

export type PiAuthProviderView = {
  id: string;
  authType: string;
};

export type PiCliResult = {
  success: boolean;
  stdout: string;
  stderr: string;
};

export function piListSessions(projectCwd?: string) {
  return invoke<PiSessionSummary[]>("pi_list_sessions", {
    projectCwd: projectCwd ?? null,
  });
}

export function piGetCwd() {
  return invoke<string>("pi_get_cwd");
}

export function piSetCwd(cwd: string) {
  return invoke<void>("pi_set_cwd", { cwd });
}

export function piNewSession(cwd?: string) {
  return invoke<unknown>("pi_new_session", { cwd: cwd ?? null });
}

export function piSwitchSession(sessionPath: string) {
  return invoke<unknown>("pi_switch_session", { sessionPath });
}

export function piPrompt(message: string) {
  return invoke<unknown>("pi_prompt", { message });
}

export function piAbort() {
  return invoke<unknown>("pi_abort");
}

export function piGetMessages() {
  return invoke<unknown>("pi_get_messages");
}

export function piReadSettings() {
  return invoke<PiSettingsView>("pi_read_settings");
}

export function piPatchSettings(opts: {
  defaultProvider?: string;
  defaultModel?: string;
  defaultThinkingLevel?: string;
}) {
  return invoke<PiSettingsView>("pi_patch_settings", {
    defaultProvider: opts.defaultProvider ?? null,
    defaultModel: opts.defaultModel ?? null,
    defaultThinkingLevel: opts.defaultThinkingLevel ?? null,
  });
}

export function piListAuthProviders() {
  return invoke<PiAuthProviderView[]>("pi_list_auth_providers");
}

export function piSetApiKey(provider: string, key: string) {
  return invoke<void>("pi_set_api_key", { provider, key });
}

export function piPackageInstall(source: string) {
  return invoke<PiCliResult>("pi_package_install", { source });
}

export function piPackageRemove(source: string) {
  return invoke<PiCliResult>("pi_package_remove", { source });
}

export function piPackageUpdate(source: string) {
  return invoke<PiCliResult>("pi_package_update", { source });
}

export function piPackageUpdateAll() {
  return invoke<PiCliResult>("pi_package_update_all");
}

export function piListProviderIds() {
  return invoke<string[]>("pi_list_provider_ids");
}

export function piListModelsStore(provider: string) {
  return invoke<unknown[]>("pi_list_models_store", { provider });
}

export function piSetModelRpc(provider: string, modelId: string) {
  return invoke<unknown>("pi_set_model_rpc", { provider, modelId });
}

export function piGetAvailableModelsRpc() {
  return invoke<unknown>("pi_get_available_models_rpc");
}

export function piGetStateRpc() {
  return invoke<unknown>("pi_get_state_rpc");
}

export function piRpc(command: Record<string, unknown>) {
  return invoke<unknown>("pi_rpc", { command });
}

export function piFork(entryId: string) {
  return piRpc({ type: "fork", entryId });
}

export const terminalStart = (cols:number,rows:number) => invoke<{id:number;cwd:string}>("terminal_start",{cols,rows});
export const terminalPoll = (id:number) => invoke<{bytes:number[];exited:boolean}>("terminal_poll",{id});
export const terminalWrite = (id:number,data:string) => invoke<void>("terminal_write",{id,data});
export const terminalResize = (id:number,cols:number,rows:number) => invoke<void>("terminal_resize",{id,cols,rows});
export const terminalClose = (id:number) => invoke<void>("terminal_close",{id});

export type CustomProvider = { id: string; baseUrl: string; api: string; models: string[] };
export type UpdateInfo = { source: string; installed: string | null; latest: string | null; available: boolean; note: string };
export const piSaveEnabledModels = (models: string[]) => invoke<void>("pi_save_enabled_models", { models });
export const piRemoveApiKey = (provider: string) => invoke<void>("pi_remove_api_key", { provider });
export const piCustomProviders = () => invoke<CustomProvider[]>("pi_custom_providers");
export const piSaveCustomProvider = (provider: string, baseUrl: string, api: string, models: string[]) => invoke<void>("pi_save_custom_provider", { provider, baseUrl, api, models });
export const piRemoveCustomProvider = (provider: string) => invoke<void>("pi_remove_custom_provider", { provider });
export const piCheckUpdate = (source?: string) => invoke<UpdateInfo>("pi_check_update", { source: source ?? null });

export function piClone() {
  return piRpc({ type: "clone" });
}

export type GitFile = { path: string; status: string };
export type GitStatus = { branch: string; root: string; files: GitFile[] };
export function piGitStatus() {
  return invoke<GitStatus>("pi_git_status");
}
export function piGitDiff(path: string, staged: boolean) {
  return invoke<string>("pi_git_diff", { path, staged });
}
export function piExtensionResponse(response: Record<string, unknown>) {
  return invoke<void>("pi_extension_response", {
    response: { ...response, type: "extension_ui_response" },
  });
}
