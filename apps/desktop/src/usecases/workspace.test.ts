import { describe, it, expect, vi } from "vitest";
import { makeWorkspaceUseCases } from "./workspace";

describe("workspace use cases", () => {
  it("creates a folder forwarding name and parent", async () => {
    const invoke = vi
      .fn()
      .mockResolvedValue({ id: "f1", parentId: null, name: "Proyecto", color: null, position: 0 });
    const uc = makeWorkspaceUseCases({ invoke });

    const folder = await uc.createFolder("Proyecto");

    expect(invoke).toHaveBeenCalledWith("folder_create", {
      name: "Proyecto",
      parentId: null,
    });
    expect(folder.id).toBe("f1");
  });

  it("persists a node position via node_set_position", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const uc = makeWorkspaceUseCases({ invoke });

    await uc.setNodePosition("n1", 42, 84);

    expect(invoke).toHaveBeenCalledWith("node_set_position", { id: "n1", x: 42, y: 84 });
  });

  it("forwards the query to node_search", async () => {
    const invoke = vi.fn().mockResolvedValue([]);
    const uc = makeWorkspaceUseCases({ invoke });

    await uc.searchNodes("web01");

    expect(invoke).toHaveBeenCalledWith("node_search", { query: "web01" });
  });

  it("normalizes optional credential fields to null", async () => {
    const invoke = vi.fn().mockResolvedValue({ id: "c1" });
    const uc = makeWorkspaceUseCases({ invoke });

    await uc.upsertCredential({ nodeId: "n1", kind: "ssh", isDefault: true });

    expect(invoke).toHaveBeenCalledWith("credential_upsert", {
      id: null,
      nodeId: "n1",
      kind: "ssh",
      username: null,
      secret: null,
      port: null,
      keyPath: null,
      isDefault: true,
      options: null,
    });
  });

  it("connects a node using the default credential when none is given", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const uc = makeWorkspaceUseCases({ invoke });

    await uc.connectNode("n1");

    expect(invoke).toHaveBeenCalledWith("connect_node", {
      nodeId: "n1",
      credentialId: null,
      contextId: null,
    });
  });

  it("connects a node with a specific credential id and context", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const uc = makeWorkspaceUseCases({ invoke });

    await uc.connectNode("n1", "c2", "vpn");

    expect(invoke).toHaveBeenCalledWith("connect_node", {
      nodeId: "n1",
      credentialId: "c2",
      contextId: "vpn",
    });
  });
});
