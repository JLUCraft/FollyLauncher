export interface Instance {
  id: string;
  name: string;
  mode: string;
  club: string;
  type: "service" | "room";
  players: string;
  latency: number | null;
  state: string;
  version: string;
  peer_id: string | null;
}
