import { describe, expect, it } from "vitest";
import { fakeName, myShownName, shownName } from "./privacy";
import { emptyData } from "./testdata";

describe("privacy mode", () => {
  it("leaves names alone when off", () => {
    const d = emptyData();
    expect(shownName(d, "Ember Fox", "usr_1")).toBe("Ember Fox");
    expect(myShownName(d, "Spiro")).toBe("Spiro");
  });

  it("gives each person a stable hidden name when on", () => {
    const d = emptyData();
    d.settings.privacyMode = true;
    const a = shownName(d, "Ember Fox", "usr_1");
    expect(a).toMatch(/^[A-Z][a-z]+ [A-Z][a-z]+( [A-Z][a-z]+)?$/);
    expect(a).not.toContain("Ember");
    expect(shownName(d, "Ember Fox", "usr_1")).toBe(a);
    // Same person by ID even if they renamed; someone else differs.
    expect(shownName(d, "New Name", "usr_1")).toBe(a);
    expect(shownName(d, "Rex", "usr_2")).not.toBe(a);
    expect(shownName(d, null)).toBeNull();
    expect(myShownName(d, "Spiro")).toBe("You");
  });

  it("rarely gives two people the same made-up name", () => {
    const names = new Set(Array.from({ length: 50 }, (_, i) => fakeName(`usr_${i}`)));
    expect(names.size).toBeGreaterThanOrEqual(48);
  });
});
