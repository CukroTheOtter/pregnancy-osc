// Debug: pretend partners (see pregsys-core/src/testbuddy.rs; keep the names
// and IDs in step). Their pictures are drawn here, never fetched.

export const TEST_BUDDIES = [
  { name: "Test Buddy", id: "usr_7e57b0d1-7e57-4e57-8e57-7e57b0d17e57", avatar: "Test Bunny", color: "#b8e0ff" },
  { name: "Test Fox", id: "usr_7e57b0d2-7e57-4e57-8e57-7e57b0d27e57", avatar: "Test Kit", color: "#ffd0a8" },
  { name: "Test Wolf", id: "usr_7e57b0d3-7e57-4e57-8e57-7e57b0d37e57", avatar: "Test Pup", color: "#cfc4ff" },
];

const svg = (color: string) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" fill="${color}"/><circle cx="32" cy="36" r="18" fill="#fff"/><ellipse cx="22" cy="14" rx="5" ry="12" fill="#fff"/><ellipse cx="42" cy="14" rx="5" ry="12" fill="#fff"/><circle cx="25" cy="34" r="2.5" fill="#333"/><circle cx="39" cy="34" r="2.5" fill="#333"/><path d="M28 41q4 4 8 0" stroke="#333" stroke-width="2" fill="none" stroke-linecap="round"/><text x="50" y="58" font-size="14" text-anchor="middle">🧪</text></svg>`;

/** A test buddy's made-up picture (null: not a test buddy). */
export function testBuddyPicture(userId: string | null | undefined): string | null {
  const b = TEST_BUDDIES.find((x) => x.id === userId);
  return b ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg(b.color))}` : null;
}

export const isTestBuddy = (userId: string | null | undefined) => TEST_BUDDIES.some((x) => x.id === userId);
