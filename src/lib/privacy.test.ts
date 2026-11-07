import { describe, expect, it } from "vitest";
import { myShownName, shownName } from "./privacy";
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
    expect(a).toMatch(/^Hidden #\d{3}$/);
    expect(shownName(d, "Ember Fox", "usr_1")).toBe(a);
    // Same person by ID even if they renamed; someone else differs.
    expect(shownName(d, "New Name", "usr_1")).toBe(a);
    expect(shownName(d, "Rex", "usr_2")).not.toBe(a);
    expect(shownName(d, null)).toBeNull();
    expect(myShownName(d, "Spiro")).toBe("You");
  });
});
