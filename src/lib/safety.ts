// Settings > Safety: VRChat's instance types, in the order Settings shows them
// (the logic lives in pregsys-core/src/safety.rs).
import type { InstanceKind } from "./types";

export const INSTANCE_KINDS: { k: InstanceKind; l: string }[] = [
  { k: "public", l: "Public" },
  { k: "groupPublic", l: "Group Public" },
  { k: "friendsPlus", l: "Friends+" },
  { k: "groupPlus", l: "Group+" },
  { k: "friends", l: "Friends" },
  { k: "group", l: "Group" },
  { k: "invitePlus", l: "Invite+" },
  { k: "invite", l: "Invite" },
];

/** Anyone can walk in. */
export const OPEN_KINDS: InstanceKind[] = ["public", "groupPublic"];

export const instanceName = (k: InstanceKind | null | undefined) => INSTANCE_KINDS.find((x) => x.k === k)?.l ?? "this";
