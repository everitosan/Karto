import { describe, expect, it } from "vitest";
import type { Credential } from "$domain/infra";
import {
  keyOnboardingReason,
  needsKeyOnboarding,
  pickCredential,
  templateConfirmCommand,
} from "./connectFlow";

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
    hasVaultKey: false,
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

describe("keyOnboardingReason", () => {
  it("password: SSH sin llave ninguna", () => {
    expect(keyOnboardingReason(cred({ keyPath: null }))).toBe("password");
  });

  // El caso que motiva la reformulación: conecta bien en este equipo, pero el
  // vault no se lleva la llave, así que el .karto no es portable.
  it("local-key: SSH con llave que sólo existe en este equipo", () => {
    expect(
      keyOnboardingReason(cred({ keyPath: "/home/me/.ssh/id", hasVaultKey: false })),
    ).toBe("local-key");
  });

  it("null si la llave ya viaja dentro del vault", () => {
    expect(
      keyOnboardingReason(cred({ keyPath: "/home/me/.ssh/id", hasVaultKey: true })),
    ).toBeNull();
  });

  // Una credencial sin keyPath pero con material en el vault: la llave se
  // materializa al conectar, así que tampoco hay nada que ofrecer.
  it("null si hay material en el vault aunque no haya ruta", () => {
    expect(keyOnboardingReason(cred({ keyPath: null, hasVaultKey: true }))).toBeNull();
  });

  it("null para credenciales no SSH", () => {
    expect(keyOnboardingReason(cred({ kind: "web", keyPath: null }))).toBeNull();
    expect(keyOnboardingReason(cred({ kind: "vnc", keyPath: null }))).toBeNull();
  });

  it("null si no hay credencial", () => {
    expect(keyOnboardingReason(undefined)).toBeNull();
  });
});

describe("needsKeyOnboarding", () => {
  it("true para SSH por contraseña (sin llave)", () => {
    expect(needsKeyOnboarding(cred({ keyPath: null }))).toBe(true);
  });

  // Detectado como `local-key`, pero todavía no se ofrece: falta el arranque con
  // la llave existente. Ofrecerlo antes sería prometer un flujo a medias.
  it("false (aún) para una llave local no portable", () => {
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

describe("templateConfirmCommand", () => {
  it("extrae el comando del aviso de confirmación de plantilla", () => {
    expect(
      templateConfirmCommand("confirmación de plantilla requerida: ssh -i /k host"),
    ).toBe("ssh -i /k host");
  });

  it("acepta el error como objeto Error", () => {
    expect(
      templateConfirmCommand(new Error("confirmación de plantilla requerida: rm -rf /")),
    ).toBe("rm -rf /");
  });

  it("devuelve null para otros errores (se propagan)", () => {
    expect(templateConfirmCommand("contraseña incorrecta")).toBeNull();
    expect(templateConfirmCommand("no hay ningún vault abierto")).toBeNull();
  });
});
