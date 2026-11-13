import { describe, expect, it } from "vitest";
import {
  filterFromSearchParams,
  filterToSearchParams,
  isSafeReturnPath,
  parseRoute,
  routeSearch,
  workspaceHref,
} from "./routes";

describe("app routes", () => {
  it("parses the seven workspaces and validates sections", () => {
    expect(parseRoute("/")).toBeNull();
    expect(parseRoute("/today")).toMatchObject({
      workspace: "today",
      section: "dashboard",
    });
    expect(parseRoute("/hands/matrix")).toMatchObject({
      workspace: "hands",
      section: "matrix",
    });
    expect(parseRoute("/learn/trainer")).toMatchObject({
      workspace: "learn",
      section: "trainer",
    });
    expect(parseRoute("/research/evidence")).toMatchObject({
      workspace: "research",
      section: "evidence",
    });
    expect(parseRoute("/coach/connections")).toMatchObject({
      workspace: "coach",
      section: "connections",
    });
    expect(parseRoute("/data/sources")).toMatchObject({
      workspace: "data",
      section: "sources",
    });
    expect(parseRoute("/settings/preferences")).toMatchObject({
      workspace: "settings",
      section: "preferences",
    });
    expect(parseRoute("/data/preferences")).toMatchObject({
      workspace: "settings",
      section: "preferences",
      pathname: "/settings/preferences",
    });
    expect(parseRoute("/hands/not-a-section")).toBeNull();
    expect(parseRoute("/not-a-workspace")).toBeNull();
  });

  it("uses a route-backed replay modal while preserving the list workspace", () => {
    const route = parseRoute("/hands/replay/123");
    expect(route).toMatchObject({
      workspace: "hands",
      section: "list",
      handId: 123,
    });
    expect(parseRoute("/hands/replay/not-a-number")).toBeNull();
  });

  it("round-trips canonical filters without unknown query values", () => {
    const filter = {
      currency: "USD",
      position: "BB",
      reviewed: false,
      stack_min: 0,
      bet_max: 75,
      date_from: "2026-11-01",
    };
    const search = routeSearch(filter, {
      dataset: "d1",
      step: 4,
      prompt: "secret",
    });
    expect(search).toBe(
      "?currency=USD&position=BB&reviewed=false&stack_min=0&bet_max=75&date_from=2026-11-01&dataset=d1&step=4",
    );
    const params = new URLSearchParams(search);
    expect(filterFromSearchParams(params)).toEqual(filter);
  });

  it("defaults currency and accepts only local return routes", () => {
    expect(filterToSearchParams({}).get("currency")).toBe("USD");
    expect(isSafeReturnPath("/coach/library")).toBe(true);
    expect(isSafeReturnPath("//evil.test")).toBe(false);
    expect(isSafeReturnPath("/hands/replay/1")).toBe(false);
    expect(workspaceHref("research", "evidence")).toBe("/research/evidence");
  });
});
