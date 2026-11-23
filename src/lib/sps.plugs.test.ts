import { describe, expect, it } from "vitest";
import { classifyPlug, plugCounts, plugsFrom } from "./sps";
import { defaultSettings } from "./testdata";
import type { AvatarProfile } from "./types";

describe("SPS plugs", () => {
  it("guesses types like sps.rs classify_plug", () => {
    // Names seen on real avatars; the same answers as sps.rs `plug_types_from_names`.
    expect(classifyPlug("Zcanine Cock")).toBe("body");
    expect(classifyPlug("Penis Body")).toBe("body");
    expect(classifyPlug("zw_ris_sps")).toBe("body");
    expect(classifyPlug("Dildo")).toBe("toy");
    expect(classifyPlug("Actual Wsome Lollipop")).toBe("toy");
    expect(classifyPlug("Tail")).toBe("other");
  });
  it("finds plugs, not sockets or your own touches", () => {
    expect(plugsFrom(["OGB/Pen/Old/PenOthers", "OGB/Pen/Old/PenSelf", "OGB/Plug/New/DepthOthers", "OGB/Orf/A/PenOthers"])).toEqual(["New", "Old"]);
  });
  it("counts by type, with per-avatar fixes", () => {
    const s = defaultSettings();
    expect(plugCounts(s, undefined, "Zcanine Cock")).toBe(true);
    expect(plugCounts(s, undefined, "Dildo")).toBe(false);
    const a = { sps: { custom: false, sockets: [], types: {}, plugTypes: { Dildo: "body" } } } as unknown as AvatarProfile;
    expect(plugCounts(s, a, "Dildo")).toBe(true);
  });
});
