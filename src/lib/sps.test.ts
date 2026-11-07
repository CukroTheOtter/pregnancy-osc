import { describe, expect, it } from "vitest";
import { socketsFrom, spsTooNew, spsVersion } from "./sps";

describe("spsVersion", () => {
  it("reads OGB/Version/<n> and spots newer ones", () => {
    expect(spsVersion(["Pregnancy/Stage", "OGB/Orf/A/PenOthers"])).toBeNull();
    expect(spsVersion(["OGB/Version/11"])).toBe(11);
    expect(spsTooNew(["OGB/Version/11"])).toBe(false);
    expect(spsTooNew(["OGB/Version/12"])).toBe(true);
    expect(spsTooNew(["OGB/Version/x", "OGB/Orf/A/Version/9"])).toBe(false);
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
