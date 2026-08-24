import { describe, it, expect, vi } from "vitest";
import { makeScriptsUseCases, type RunEvent } from "./scripts";

describe("scripts use cases", () => {
  it("lists the machine-level library", async () => {
    const invoke = vi.fn().mockResolvedValue([]);
    const uc = makeScriptsUseCases({ invoke });

    await uc.list();

    expect(invoke).toHaveBeenCalledWith("script_list");
  });

  it("forwards upsert and delete", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const uc = makeScriptsUseCases({ invoke });

    await uc.upsert({ name: "Disco", body: "df -h", interpreter: "bash" });
    expect(invoke).toHaveBeenCalledWith("script_upsert", {
      name: "Disco",
      body: "df -h",
      interpreter: "bash",
    });

    await uc.remove("scr-1");
    expect(invoke).toHaveBeenCalledWith("script_delete", { id: "scr-1" });
  });

  it("forwards folder CRUD and moving a script", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const uc = makeScriptsUseCases({ invoke });

    await uc.listFolders();
    expect(invoke).toHaveBeenCalledWith("script_folder_list");

    await uc.createFolder("Redes");
    expect(invoke).toHaveBeenCalledWith("script_folder_create", { name: "Redes" });

    await uc.renameFolder("f1", "Red");
    expect(invoke).toHaveBeenCalledWith("script_folder_rename", { id: "f1", name: "Red" });

    await uc.removeFolder("f1");
    expect(invoke).toHaveBeenCalledWith("script_folder_delete", { id: "f1" });

    await uc.setFolder("scr-1", "f1");
    expect(invoke).toHaveBeenCalledWith("script_set_folder", { id: "scr-1", folderId: "f1" });

    await uc.setFolder("scr-1", null);
    expect(invoke).toHaveBeenCalledWith("script_set_folder", { id: "scr-1", folderId: null });
  });

  it("requests targets for a diagram", async () => {
    const invoke = vi.fn().mockResolvedValue([]);
    const uc = makeScriptsUseCases({ invoke });

    await uc.targets("map-1");

    expect(invoke).toHaveBeenCalledWith("script_targets", { mapId: "map-1" });
  });

  it("wires the channel and forwards the run request with its events", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    // Canal falso: captura el handler y permite emitir eventos manualmente.
    const fakeChannel = { onmessage: undefined as ((ev: RunEvent) => void) | undefined };
    const uc = makeScriptsUseCases({ invoke }, () => fakeChannel as never);

    const seen: RunEvent[] = [];
    await uc.run(
      { nodeIds: ["n1"], body: "uname -a", interpreter: "bash", mode: "parallel", contextId: "vpn" },
      (ev) => seen.push(ev),
    );

    // El canal viaja como `onEvent` junto al resto de argumentos.
    expect(invoke).toHaveBeenCalledWith("scripts_run", {
      nodeIds: ["n1"],
      body: "uname -a",
      interpreter: "bash",
      mode: "parallel",
      contextId: "vpn",
      onEvent: fakeChannel,
    });

    // Los mensajes del canal llegan al callback.
    fakeChannel.onmessage?.({ type: "line", nodeId: "n1", line: "Linux" });
    expect(seen).toEqual([{ type: "line", nodeId: "n1", line: "Linux" }]);
  });
});
