export type MdaFilter = {
  date_from?: string;
  date_to?: string;
  bb_units?: number;
  position?: string;
  opponent?: string;
  street?: string;
  pot_type?: string;
  players?: number;
  hu_postflop?: boolean;
  effective_min?: number;
  effective_max?: number;
  bet_min?: number;
  bet_max?: number;
  board?: string;
  texture?: string;
  paired?: boolean;
  high_card?: string;
  metric?: string;
};
export type MdaContext = {
  dataset_id: string;
  version: string;
  filter: MdaFilter;
};
export type MdaDataset = {
  id: string;
  name: string;
  pool: string;
  timezone: string;
  cloud_enabled: boolean;
  mcp_enabled: boolean;
};
export type MdaCatalog = {
  dataset: MdaDataset;
  version: string;
  hands: number;
  quarantined: number;
  stakes: {
    bb_units: number;
    hands: number;
    date_min: number;
    date_max: number;
  }[];
  metrics: { key: string; definition: string }[];
  error?: string;
};
export type MdaStat = {
  metric: string;
  bb_units: number;
  street: string;
  context: string;
  numerator: number;
  opportunities: number;
  pct: number;
  wilson95: [number, number];
  period_interval_overlap: boolean | null;
  classification?: string;
  adjustment?: string;
  showdown_coverage_pct: number | null;
  periods: { opportunities: number; numerator: number }[];
};
export type MdaSpot = {
  hand_key: string;
  seq: number;
  seat: number;
  day: number;
  position: string;
  opponent: string;
  street: string;
  pot_type: string;
  players: number;
  board: string;
  action: string;
  effective_bb: number;
  facing_bet_pct: number | null;
};
export type MdaJob = {
  id: string;
  state: string;
  scanned: number;
  inserted: number;
  duplicates: number;
  quarantined: number;
  conflicts: number;
  source_index: number;
  sources: { path: string; member?: string }[];
  elapsed_ms: number;
  message?: string;
};
export type MdaRequest =
  | { op: "list" }
  | { op: "create"; name: string; timezone: string; source: string }
  | {
      op: "share";
      dataset_id: string;
      cloud_enabled: boolean;
      mcp_enabled: boolean;
    }
  | { op: "start_import"; dataset_id: string; paths: string[] }
  | { op: "pause"; dataset_id: string }
  | { op: "resume"; dataset_id: string; job_id: string }
  | { op: "jobs"; dataset_id: string }
  | {
      op: "query";
      dataset_id: string;
      name: string;
      arguments: Record<string, unknown>;
    }
  | { op: "hand"; dataset_id: string; hand_key: string; version: string }
  | { op: "issues"; dataset_id: string; before?: number }
  | { op: "issue_raw"; dataset_id: string; id: number }
  | { op: "backup"; dataset_id: string; path: string }
  | { op: "restore"; path: string };
