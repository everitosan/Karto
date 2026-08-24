import { describe, it, expect, vi } from "vitest";
import { makeRecentsUseCases } from "./recents";

describe("recents use cases", () => {
  it("list() invokes recents_list and returns the entries", async () => {
    const entries = [{ path: "/a.karto", lastOpened: 2 }];
    const invoke = vi.fn().mockResolvedValue(entries);
    const useCases = makeRecentsUseCases({ invoke });

    const out = await useCases.list();

    expect(invoke).toHaveBeenCalledWith("recents_list");
    expect(out).toEqual(entries);
  });

  it("forget() forwards the path to recents_forget", async () => {
    const invoke = vi.fn().mockResolvedValue([]);
    const useCases = makeRecentsUseCases({ invoke });

    await useCases.forget("/a.karto");

    expect(invoke).toHaveBeenCalledWith("recents_forget", { path: "/a.karto" });
  });

  it("defaultVaultDir() invokes default_vault_dir", async () => {
    const invoke = vi.fn().mockResolvedValue("/home/user");
    const useCases = makeRecentsUseCases({ invoke });

    const dir = await useCases.defaultVaultDir();

    expect(invoke).toHaveBeenCalledWith("default_vault_dir");
    expect(dir).toBe("/home/user");
  });
});
