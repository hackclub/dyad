export interface Me {
  first_name: string;
  friend_code: number;
  invite_token: string;
  hackatime_linked: boolean;
  ysws_eligible: boolean;
  verification_status: string;
  reminders: boolean;
  dms: boolean;
}

export type HackatimeStatus = "ok" | "unlinked" | "expired" | "unreachable";

export interface Member {
  user_id: number;
  name: string;
  hours_logged: number | null;
  hours_needed: number;
  hours_target: number;
  next_credits_at: number;
  first_time: boolean;
  confirmed: boolean;
  ready_to_ship: boolean;
  submitting: boolean;
  hackatime: HackatimeStatus;
}

export interface JoinRequest {
  user_id: number;
  name: string;
}

export interface OutgoingRequest {
  project_id: number;
  project_name: string;
  owner_name: string;
}

export interface ProjectInfo {
  project_id: number;
  name: string;
  description: string;
  screenshot_url: string;
  code_url: string;
  playable_url: string;
  games: string[];
  shipped: boolean;
  review_status: "pending" | "processing" | "approved" | "error" | null;
  hackatime_project_names: string[];
  credit_budget: number;
  credits_earned: number;
  credits_spent: number;
  credits_per_block: number;
  me: Member;
  friend: Member | null;
  join_requests: JoinRequest[];
}

export interface Dashboard {
  hackatime: HackatimeStatus;
  projects: ProjectInfo[];
  join_requests: OutgoingRequest[];
}

export interface Invite {
  name: string;
  project_name: string | null;
}

export interface PartnerInvite {
  name: string;
  token: string;
}

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
  }
}

export interface Game {
  name: string;
  credits: number;
  copies_label: string;
  image_url: string;
}

export interface GameCatalog {
  credits_per_block: number;
  games: Game[];
}

export interface HackatimeProject {
  name: string;
  hours: number;
  unusable: string | null;
}

export async function api<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method: body === undefined ? "GET" : "POST",
    headers: body === undefined ? {} : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (res.status === 401) {
    window.location.href = "/login";
    throw new Error("Not signed in");
  }
  if (!res.ok) {
    const data = await res.json().catch(() => null);
    throw new ApiError(data?.error ?? `Request failed (${res.status})`, res.status);
  }
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

export interface StatsHour {
  hour: string;
  signed_up: number;
  projects_created: number;
  shipped: number;
}

export interface Funnel {
  signed_in: number;
  linked_hackatime: number;
  created_project: number;
  paired: number;
  logged_time: number;
  shipped: number;
  approved: number;
}

export interface Stats {
  hours: StatsHour[];
  totals: {
    hours_building: number;
    projects_created: number;
    pairs_formed: number;
    projects_shipped: number;
    projects_approved: number;
    hours_approved: number;
    logged_in_last_day: number;
    awaiting_review: number;
    average_wait_hours: number | null;
    average_approval_hours: number | null;
  };
  funnel: Record<"day" | "week" | "all", Funnel>;
}
