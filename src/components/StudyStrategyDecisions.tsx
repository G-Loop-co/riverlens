import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Filter } from "../types";
import type { SpotReport } from "../study-types";
import { study, useStudy, useStudyMutation } from "../study-api";
import { ErrorBanner } from "./UI";

export function StrategyDecisions({pack, node, hand, filter, revision, open, notify, refresh}: {
  pack: string; node: string; hand: string; filter: Filter; revision: number;
  open: (id: number, step: number) => void; notify: (s: string) => void; refresh: () => void;
}) {
  const { t } = useTranslation();
  const [matched, setMatched] = useState(true);
  const [cellOnly, setCellOnly] = useState(false);
  const [before, setBefore] = useState<number | null>(null);
  const key = JSON.stringify({pack, node, hand, filter, matched, cellOnly, revision});
  const [pageKey, setPageKey] = useState(key);
  useEffect(() => { setBefore(null); setPageKey(key); }, [key]);
  const data = useStudy<SpotReport>(pack && node ? {op: "explore", query: {
    filter: {...filter, ...(cellOnly ? {hand_class: hand} : {})}, spot: {line: []},
    strategy: {pack, node, matched_only: matched}, before: pageKey === key ? before : null,
  }} : null, revision);
  const mutation = useStudyMutation();
  return <section className="panel study-builder">
    <h3>{t("study.viewDecisions")}</h3>
    <div className="study-row-actions">
      <label><input type="checkbox" checked={matched} onChange={e => setMatched(e.target.checked)}/>{t("study.matchedOnly")}</label>
      <label><input type="checkbox" checked={cellOnly} onChange={e => setCellOnly(e.target.checked)}/>{t("study.selectedClass")} {hand}</label>
    </div>
    <p className="muted">{t(matched ? "study.matchedCohort" : "study.referenceCohort")} · {data.data?.opportunities ?? "—"}</p>
    {(data.error || mutation.error) && <ErrorBanner>{data.error || mutation.error}</ErrorBanner>}
    <div className="table-scroll"><table className="data-table"><thead><tr>
      <th>{t("study.decision")}</th><th>{t("study.observed")}</th><th>{t("study.tools")}</th>
    </tr></thead><tbody>{data.data?.rows.map(row => <tr key={row.id}>
      <td>{row.decision.reference.hand_id} · {row.decision.hand_class} · #{row.decision.reference.seq}</td>
      <td>{t(`study.${row.decision.action}`)} {row.decision.size_bb === null ? "" : `${row.decision.size_bb}bb`}</td>
      <td><button className="text-button" onClick={() => open(row.hand, row.decision.reference.seq)}>{t("study.replay")}</button>
      <button className="text-button" disabled={mutation.busy} onClick={() => void mutation.run(async () => {
        const added = await study<{added: number; duplicates: number}>({op:"enqueue", references:[row.decision.reference], pack});
        notify(t("study.added", added)); refresh();
      })}>{t("study.train")}</button></td>
    </tr>)}</tbody></table></div>
    {data.data?.rows.length === 0 && <p>{t("study.noDecisions")}</p>}
    <div className="study-row-actions">
      {before !== null && <button className="button subtle" onClick={() => setBefore(null)}>{t("study.firstPage")}</button>}
      {data.data?.next_cursor != null && <button className="button subtle" onClick={() => setBefore(data.data!.next_cursor)}>{t("study.loadMore")}</button>}
    </div>
  </section>;
}
