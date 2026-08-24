import { describe, it, expect } from "vitest";
import { hostTarget, isDuplicate } from "./sshImport";
import type { ImportedHost, InfraNode } from "$domain/infra";

const host = (over: Partial<ImportedHost> = {}): ImportedHost => ({
  alias: "web1",
  hostname: "10.0.0.10",
  user: "deploy",
  port: 22,
  identityFile: null,
  ...over,
});

const node = (over: Partial<InfraNode> = {}): InfraNode => ({
  id: "n1",
  mapId: "m1",
  kind: "server",
  label: "web1",
  x: 0,
  y: 0,
  parentId: null,
  properties: {},
  endpoints: {},
  ...over,
});

describe("hostTarget", () => {
  it("prefers hostname over alias", () => {
    expect(hostTarget(host())).toBe("10.0.0.10");
  });
  it("falls back to alias when hostname is null", () => {
    expect(hostTarget(host({ hostname: null }))).toBe("web1");
  });
});

describe("isDuplicate", () => {
  it("matches by label equal to alias", () => {
    expect(isDuplicate(host(), [node({ label: "web1" })])).toBe(true);
  });
  it("matches by hostname property equal to target", () => {
    expect(
      isDuplicate(host({ alias: "otro" }), [
        node({ label: "otro-label", properties: { hostname: "10.0.0.10" } }),
      ]),
    ).toBe(true);
  });
  it("matches by endpoint address (any context)", () => {
    expect(
      isDuplicate(host(), [node({ label: "x", endpoints: { default: "10.0.0.10" } })]),
    ).toBe(true);
  });
  it("is false when nothing overlaps", () => {
    expect(
      isDuplicate(host(), [node({ label: "db1", endpoints: { default: "1.1.1.1" } })]),
    ).toBe(false);
  });
});
