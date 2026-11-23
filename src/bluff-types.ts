import type { Filter } from "./types";

export interface BluffQuery {
  filter: Filter;
  view: string;
  breakdown: string;
  brand?: string;
  players?: string;
  action?: string;
  sizing?: string;
  draw?: string;
  opponent?: string;
  depth?: string;
  texture?: string;
  cell?: { category: string; bucket: string };
  before?: [number, number];
}

export interface BluffMoney {
  segment_bb: number | null;
  net_bb: number;
  segment: string | null;
  net: string | null;
  currency: string | null;
}
export interface BluffGroup extends BluffMoney {
  category: string;
  bucket: string;
  attempts: number;
  hands: number;
  folded: number;
  settled: number;
}
export interface BluffSummary {
  groups: BluffGroup[];
  coverage: { total: number; indexed: number; complete: boolean };
  unknown: number;
  exclusions: Record<string, number>;
  stakes: string[];
  currencies: string[];
}
export interface BluffHand extends BluffMoney {
  followups?: { seq: number; action: string; size: number | null }[];
  id: number;
  hand_id: string;
  seq: number;
  start: number;
  date: string;
  street: string;
  holding: string;
  outs: number;
  action: string;
  response: string;
  exclusion: string | null;
  size: number | null;
  players: number;
  gutshot: boolean;
  missed: boolean;
}
export interface BluffPage {
  rows: BluffHand[];
  next: [number, number] | null;
}
