import { invoke } from "@tauri-apps/api/core";

export async function store(
  service: string,
  user: string,
  value: string,
  opts: { biometric?: boolean } = {},
): Promise<void> {
  await invoke("plugin:keystore|store", {
    payload: { service, user, value, biometric: opts.biometric ?? true },
  });
}

export async function retrieve(
  service: string,
  user: string,
): Promise<string | null> {
  const r = await invoke<{ value: string | null }>("plugin:keystore|retrieve", {
    payload: { service, user },
  });
  return r.value ?? null;
}

export async function remove(service: string, user: string): Promise<void> {
  await invoke("plugin:keystore|remove", { payload: { service, user } });
}
