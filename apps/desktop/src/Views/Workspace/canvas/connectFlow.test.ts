import { describe, expect, it } from "vitest";
import type { Credential } from "$domain/infra";
import { needsKeyOnboarding, pickCredential } from "./connectFlow";

function cred(overrides: Partial<Credential> = {}): Credential {
  return {
    id: "c1",
    nodeId: "n1",
    kind: "ssh",
    username: "root",
    port: 22,
    keyPath: null,
    isDefault: true,
    options: null,
    extras: "{}",
    ...overrides,
  };
}

describe("pickCredential", () => {
  it("devuelve la credencial por id", () => {
    const list = [cred({ id: "a", isDefault: false }), cred({ id: "b" })];
    expect(pickCredential(list, "a")?.id).toBe("a");
  });

  it("con id nulo prefiere la predeterminada", () => {
    const list = [cred({ id: "a", isDefault: false }), cred({ id: "b", isDefault: true })];
    expect(pickCredential(list, null)?.id).toBe("b");
  });

  it("con id nulo y sin predeterminada usa la primera", () => {
    const list = [cred({ id: "a", isDefault: false }), cred({ id: "b", isDefault: false })];
    expect(pickCredential(list, null)?.id).toBe("a");
  });
});

describe("needsKeyOnboarding", () => {
  it("true para SSH por contraseña (sin llave)", () => {
    expect(needsKeyOnboarding(cred({ keyPath: null }))).toBe(true);
  });

  it("false si la credencial SSH ya tiene llave", () => {
    expect(needsKeyOnboarding(cred({ keyPath: "/home/me/.ssh/id" }))).toBe(false);
  });

  it("false para credenciales no SSH", () => {
    expect(needsKeyOnboarding(cred({ kind: "web", keyPath: null }))).toBe(false);
    expect(needsKeyOnboarding(cred({ kind: "vnc", keyPath: null }))).toBe(false);
  });

  it("false si no hay credencial", () => {
    expect(needsKeyOnboarding(undefined)).toBe(false);
  });
});
