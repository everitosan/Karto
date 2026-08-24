import { describe, it, expect, vi } from "vitest";
import { makeTemplatesUseCases } from "./templates";

describe("templates use cases", () => {
  it("list() invokes template_list", async () => {
    const invoke = vi.fn().mockResolvedValue([]);
    await makeTemplatesUseCases({ invoke }).list();
    expect(invoke).toHaveBeenCalledWith("template_list");
  });

  it("upsert() forwards the input as-is", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const input = { name: "T", connection: "ssh", command: "ssh {userhost}" };
    await makeTemplatesUseCases({ invoke }).upsert(input);
    expect(invoke).toHaveBeenCalledWith("template_upsert", input);
  });

  it("linkToVault() passes the template id", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    await makeTemplatesUseCases({ invoke }).linkToVault("tpl-1");
    expect(invoke).toHaveBeenCalledWith("template_link_to_vault", { id: "tpl-1" });
  });

  it("unlink() passes the connection kind", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    await makeTemplatesUseCases({ invoke }).unlink("ssh");
    expect(invoke).toHaveBeenCalledWith("template_vault_unlink", { connection: "ssh" });
  });
});
