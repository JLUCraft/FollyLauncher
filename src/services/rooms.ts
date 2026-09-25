import { invoke } from "@tauri-apps/api/core";
import { GameConnection } from "@/services/union";

export interface Room {
  peer: string;
  service: string;
  name: string;
  addresses: string[];
  metadata: { school: string; version: string; players_max: number };
}
export class RoomService {
  static discover(
    seeds: string[]
  ): Promise<{ rooms: Room[]; unavailable: string[] }> {
    return invoke("union_rooms", { seeds });
  }
  static join(room: Room): Promise<GameConnection> {
    return invoke("union_proxy_start", {
      input: {
        peer: room.peer,
        service: room.service,
        addresses: room.addresses,
        origin: "room",
      },
    });
  }
  static host(
    configPath: string,
    seeds: string[],
    relays: string[]
  ): Promise<Room> {
    return invoke("union_room_host", {
      configPath,
      seeds,
      relays,
    });
  }
  static hostIdentity(): Promise<string> {
    return invoke("union_room_identity");
  }
  static stop(): Promise<void> {
    return invoke("union_room_stop");
  }
  static status(): Promise<Room | null> {
    return invoke("union_room_status");
  }
}
