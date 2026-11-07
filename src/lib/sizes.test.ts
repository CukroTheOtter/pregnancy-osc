import { describe, expect, it } from "vitest";
import { babySize } from "./sizes";

describe("babySize", () => {
  it("is a spark before week 4", () => {
    expect(babySize(0)).toBe("✨ Just a tiny spark so far");
    expect(babySize(3.9)).toBe("✨ Just a tiny spark so far");
  });
  it("holds a size until the next one", () => {
    expect(babySize(7)).toBe("🫐 About the size of a blueberry");
    expect(babySize(7.8)).toBe("🫐 About the size of a blueberry");
    expect(babySize(30)).toBe("🥥 About the size of a coconut");
    expect(babySize(45)).toBe("🍉 About the size of a watermelon");
  });
  it("says an before a vowel, and each for more than one", () => {
    expect(babySize(14)).toBe("🍎 About the size of an apple");
    expect(babySize(16, true)).toBe("🥑 Each about the size of an avocado");
  });
});
