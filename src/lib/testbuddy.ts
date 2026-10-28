// Debug: the pretend partner "Test Buddy" (see pregsys-core/src/testbuddy.rs;
// keep the name and ID in step). Their picture is drawn here, never fetched.

export const TEST_BUDDY_NAME = "Test Buddy";
export const TEST_BUDDY_ID = "usr_7e57b0d1-7e57-4e57-8e57-7e57b0d17e57";

const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" fill="#b8e0ff"/><circle cx="32" cy="36" r="18" fill="#fff"/><ellipse cx="22" cy="14" rx="5" ry="12" fill="#fff"/><ellipse cx="42" cy="14" rx="5" ry="12" fill="#fff"/><circle cx="25" cy="34" r="2.5" fill="#333"/><circle cx="39" cy="34" r="2.5" fill="#333"/><path d="M28 41q4 4 8 0" stroke="#333" stroke-width="2" fill="none" stroke-linecap="round"/><text x="50" y="58" font-size="14" text-anchor="middle">🧪</text></svg>`;

export const TEST_BUDDY_PICTURE = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;

export const isTestBuddy = (userId: string | null | undefined) => userId === TEST_BUDDY_ID;
