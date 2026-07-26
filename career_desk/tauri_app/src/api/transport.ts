import { invoke } from "@tauri-apps/api/core";
import type { CommandMap, CommandName } from "./contracts";

export interface CommandTransport {
  call<K extends CommandName>(command: K, request: CommandMap[K]["request"]): Promise<CommandMap[K]["response"]>;
}

/** The only frontend module allowed to touch Tauri's raw invoke API. */
export const tauriTransport: CommandTransport = {
  call(command, request) {
    const args = command === "parse_jd" ? request : { payload: request };
    return invoke<CommandMap[typeof command]["response"]>(command, args as import("@tauri-apps/api/core").InvokeArgs | undefined);
  }
};
