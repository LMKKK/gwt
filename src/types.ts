export type Availability = "available" | "prunable" | "bare" | "unavailable";

export interface WorktreeRecord {
  path: string;
  head: string;
  branch?: string;
  detached: boolean;
  bare: boolean;
  locked?: string;
  prunable?: string;
}

export interface StatusCounts {
  conflicted: number;
  staged: number;
  modified: number;
  untracked: number;
}

export interface WorktreeView extends WorktreeRecord {
  current: boolean;
  availability: Availability;
  status?: StatusCounts;
}
