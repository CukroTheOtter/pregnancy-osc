import { describe, expect, it } from "vitest";
import { socketsFrom, spsNewerText, spsTooNew, spsVersion } from "./sps";

describe("spsVersion", () => {
  it("reads OGB/Version/<n> and spots newer ones", () => {
    expect(spsVersion(["Pregnancy/Stage", "OGB/Orf/A/PenOthers"])).toBeNull();
    expect(spsVersion(["OGB/Version/11"])).toBe(11);
    expect(spsTooNew(["OGB/Version/11"])).toBe(false);
    expect(spsTooNew(["OGB/Version/12"])).toBe(true);
    expect(spsTooNew(["OGB/Version/x", "OGB/Orf/A/Version/9"])).toBe(false);
    expect(spsVersion(["VFH/Version/10"])).toBe(10);
    expect(spsTooNew(["VFH/Version/12"])).toBe(true);
  });

  it("treats unknown OGB parameters with no readable sockets as newer", () => {
    expect(spsTooNew(["OGB/Hole/Front/Depth", "Pregnancy/Stage"])).toBe(true);
    expect(spsTooNew(["OGB/Plug/Toy/DepthOthers", "OGB/Touch/Hand/Others", "OGB/Pen/Old/PenOthers", "OGB/Enabled", "OGB/HapticsDisabled"])).toBe(false);
    expect(spsTooNew(["OGB/Socket/A/DepthOthers", "OGB/Extra/Thing"])).toBe(false);
    expect(spsTooNew(["Pregnancy/Stage", "OGB_ENABLED"])).toBe(false);
  });

  it("says the version when it can", () => {
    expect(spsNewerText(["OGB/Version/12"])).toBe("a newer SPS (version 12) than this app knows (11)");
    expect(spsNewerText(["OGB/Hole/A/B"])).toBe("SPS parameters this app doesn't know yet (probably a newer SPS)");
  });
});

describe("socketsFrom", () => {
  it("finds old and SPS3 sockets, once each", () => {
    expect(
      socketsFrom([
        "OGB/Orf/Pussy/PenOthersNewRoot",
        "OGB/Orf/Pussy/PenSelfNewRoot",
        "OGB/Socket/Front_Socket/DepthOthers",
        "OGB/Socket/Front_Socket/DepthOthers_PointSeparation",
        "OGB/Socket/Front_Socket/DepthSelf",
        "OGB/Socket/Mouth/TouchOthers",
        "OGB/Plug/Toy/DepthOthers",
        "Pregnancy/Stage",
      ]),
    ).toEqual(["Front_Socket", "Pussy"]);
  });
});
