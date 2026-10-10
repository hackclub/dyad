export interface Me {
  first_name: string;
  friend_code: number;
  hackatime_linked: boolean;
  ysws_eligible: boolean;
  verification_status: string;
  reminders: boolean;
}

export interface Member {
  name: string;
  hours_logged: number;
  hours_needed: number;
  first_time: boolean;
  confirmed: boolean;
  ready_to_ship: boolean;
  hackatime_linked: boolean;
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
  me: Member;
  friend: Member | null;
}

export interface Invite {
  name: string;
  project_name: string | null;
}

export interface Game {
  name: string;
  credits: number;
  copies_label: string;
  image_url: string;
}

export interface GameCatalog {
  credit_budget: number;
  games: Game[];
}

export interface HackatimeProject {
  name: string;
  hours: number;
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
    throw new Error(data?.error ?? `Request failed (${res.status})`);
  }
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}
