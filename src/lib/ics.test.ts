import { describe, expect, it } from "vitest";
import { escapeText, toIcs } from "./ics";

describe("ics", () => {
  it("writes all-day events that end the day after", () => {
    const text = toIcs([{ uid: "a", start: new Date(2026, 10, 1), end: new Date(2026, 10, 3), title: "Pregnant" }], new Date(Date.UTC(2026, 10, 10, 12)));
    expect(text).toContain("DTSTART;VALUE=DATE:20261101");
    expect(text).toContain("DTEND;VALUE=DATE:20261104");
    expect(text).toContain("DTSTAMP:20261110T120000Z");
    expect(text.startsWith("BEGIN:VCALENDAR\r\n")).toBe(true);
    expect(text.endsWith("END:VCALENDAR\r\n")).toBe(true);
  });

  it("escapes text and folds long lines", () => {
    expect(escapeText("a,b;c\\d\ne")).toBe("a\\,b\\;c\\\\d\\ne");
    const text = toIcs([{ uid: "b", start: new Date(2026, 0, 1), end: new Date(2026, 0, 1), title: "x".repeat(100) }]);
    for (const line of text.split("\r\n")) expect([...line].length).toBeLessThanOrEqual(75);
  });
});
