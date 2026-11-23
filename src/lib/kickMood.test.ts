import { describe, expect, it } from "vitest";
import { hashText, kickMood, moodFor } from "./kickMood";

describe("baby personality", () => {
  it("matches movement.rs", () => {
    // The same IDs and answers as movement.rs `same_moods_as_the_ui`.
    expect(hashText("preg-abc")).toBe(6813124331427210414n);
    expect(["preg-abc", "preg-2", "preg-3", "preg-4"].map(moodFor)).toEqual(["sleepy", "wiggly", "bouncy", "calm"]);
  });
  it("a picked one wins", () => {
    expect(kickMood({ id: "preg-abc", kickMood: "bouncy" })).toBe("bouncy");
    expect(kickMood({ id: "preg-abc", kickMood: null })).toBe("sleepy");
  });
});
