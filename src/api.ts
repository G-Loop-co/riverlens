import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Request } from "./types";
import i18n from "./i18n";
import { diagnosticMessage } from "./diagnostics";
export const desktop = isTauri();

export async function api<T>(request: Request): Promise<T> {
  if (desktop) return invoke<T>("api", { request });
  const response = await fetch("/api", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  const body = await response.json();
  if (!response.ok || body.error)
    throw new Error(body.error || i18n.t("本機引擎未能回應"));
  return body.result as T;
}
export function errorText(error: unknown): string {
  return diagnosticMessage(
    error instanceof Error ? error.message : String(error),
  );
}
export function useRpc<T>(request: Request | null, revision = 0) {
  const key = JSON.stringify(request);
  const [state, setState] = useState<{
    key: string;
    data: T | null;
    loading: boolean;
    error: string;
  }>({ key: "null", data: null, loading: false, error: "" });
  useEffect(() => {
    let active = true;
    if (key === "null") {
      setState({ key, data: null, loading: false, error: "" });
      return;
    }
    setState((previous) => ({
      key,
      data: previous.key === key ? previous.data : null,
      loading: true,
      error: "",
    }));
    api<T>(JSON.parse(key))
      .then((value) => {
        if (active) setState({ key, data: value, loading: false, error: "" });
      })
      .catch((e) => {
        if (active)
          setState({ key, data: null, loading: false, error: errorText(e) });
      });
    return () => {
      active = false;
    };
  }, [key, revision]);
  return state.key === key
    ? state
    : { key, data: null, loading: key !== "null", error: "" };
}
export async function chooseInput(directory = false): Promise<string[]> {
  const result = await open({
    directory,
    multiple: !directory,
    filters: directory
      ? undefined
      : [{ name: i18n.t("牌譜"), extensions: ["txt", "zip"] }],
  });
  return result ? (Array.isArray(result) ? result : [result]) : [];
}
export async function chooseOutput(
  extension: string,
  name: string,
): Promise<string | null> {
  return save({
    defaultPath: name,
    filters: [{ name: extension.toUpperCase(), extensions: [extension] }],
  });
}
