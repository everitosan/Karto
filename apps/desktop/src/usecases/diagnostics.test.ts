import { describe, it, expect, vi } from "vitest";
import { makeDiagnosticsUseCases, normalizeLevel } from "./diagnostics";

describe("diagnostics normalizeLevel", () => {
  it("passes through valid levels and falls back to warning", () => {
    expect(normalizeLevel("info")).toBe("info");
    expect(normalizeLevel("error")).toBe("error");
    expect(normalizeLevel("warning")).toBe("warning");
    expect(normalizeLevel("nope")).toBe("warning");
    expect(normalizeLevel(undefined)).toBe("warning");
    expect(normalizeLevel(null)).toBe("warning");
  });
});

describe("diagnostics use cases", () => {
  it("getLevel() normalizes the backend value", async () => {
    const invoke = vi.fn().mockResolvedValue("garbage");
    const useCases = makeDiagnosticsUseCases({ invoke });

    const level = await useCases.getLevel();

    expect(invoke).toHaveBeenCalledWith("log_level_get");
    expect(level).toBe("warning");
  });

  it("setLevel() forwards the chosen level", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const useCases = makeDiagnosticsUseCases({ invoke });

    await useCases.setLevel("error");

    expect(invoke).toHaveBeenCalledWith("log_level_set", { level: "error" });
  });

  it("openLogDir() and logPath() call their commands", async () => {
    const invoke = vi.fn().mockResolvedValue("/data/karto.log");
    const useCases = makeDiagnosticsUseCases({ invoke });

    expect(await useCases.logPath()).toBe("/data/karto.log");
    expect(invoke).toHaveBeenCalledWith("log_path_get");

    await useCases.openLogDir();
    expect(invoke).toHaveBeenCalledWith("open_log_dir");
  });
});
