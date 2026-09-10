import { useRef, useState } from "react";
import { api, errorText, useRpc } from "./api";
import type { StudyRequest } from "./study-types";
export const study = <T>(request: StudyRequest) =>
  api<T>({ op: "study", request });
export const useStudy = <T>(request: StudyRequest | null, revision = 0) =>
  useRpc<T>(request ? { op: "study", request } : null, revision);
export function useStudyMutation() {
  const gate = useRef(false);
  const [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  async function run(action: () => Promise<void>) {
    if (gate.current) return;
    gate.current = true;
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (e) {
      setError(errorText(e));
    } finally {
      gate.current = false;
      setBusy(false);
    }
  }
  return { busy, error, run };
}
