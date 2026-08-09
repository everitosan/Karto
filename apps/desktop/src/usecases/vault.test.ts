import { describe, it, expect, vi } from "vitest";
import { VaultError } from "$domain/vault";
import { makeVaultUseCases } from "./vault";

describe("vault use cases", () => {
  it("forwards create() args to the vault_create command", async () => {
    const invoke = vi.fn().mockResolvedValue({ path: "/x.karto", status: "unlocked" });
    const useCases = makeVaultUseCases({ invoke });

    const info = await useCases.create({ path: "/x.karto", password: "s3cret!!" });

    expect(invoke).toHaveBeenCalledWith("vault_create", {
      path: "/x.karto",
      password: "s3cret!!",
    });
    expect(info.status).toBe("unlocked");
  });

  it("maps a wrong-password backend error to VaultError kind", async () => {
    const invoke = vi.fn().mockRejectedValue("wrong password / HMAC check failed");
    const useCases = makeVaultUseCases({ invoke });

    await expect(
      useCases.unlock({ path: "/x.karto", password: "nope" }),
    ).rejects.toMatchObject({ name: "VaultError", kind: "wrong-password" });
  });

  it("wraps unknown errors as VaultError", async () => {
    const invoke = vi.fn().mockRejectedValue("disk exploded");
    const useCases = makeVaultUseCases({ invoke });

    const error = await useCases.create({ path: "/x", password: "12345678" }).catch((e) => e);
    expect(error).toBeInstanceOf(VaultError);
    expect((error as VaultError).kind).toBe("unknown");
  });
});
