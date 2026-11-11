import { describe, expect, it } from "vitest";
import { parseOrchestratorCommand } from "./OrchestratorBar";

describe("parseOrchestratorCommand", () => {
  it("ignores plain text so it never reaches the PTY as an app command", () => {
    expect(parseOrchestratorCommand("git status")).toEqual({ kind: "none" });
    expect(parseOrchestratorCommand("")).toEqual({ kind: "none" });
  });

  it("parses agent switch commands case-insensitively", () => {
    expect(parseOrchestratorCommand("///agente Claude")).toEqual({ kind: "switch", agentId: "claude" });
    expect(parseOrchestratorCommand("///agent opencode")).toEqual({ kind: "switch", agentId: "opencode" });
    expect(parseOrchestratorCommand("///agente")).toEqual({ kind: "switch", agentId: "" });
  });

  it("parses resume synonyms", () => {
    expect(parseOrchestratorCommand("///retomar")).toEqual({ kind: "resume" });
    expect(parseOrchestratorCommand("///resume")).toEqual({ kind: "resume" });
    expect(parseOrchestratorCommand("///continuar")).toEqual({ kind: "resume" });
  });

  it("parses help and reports unknown commands distinctly", () => {
    expect(parseOrchestratorCommand("///ajuda")).toEqual({ kind: "help" });
    expect(parseOrchestratorCommand("///")).toEqual({ kind: "help" });
    expect(parseOrchestratorCommand("///voar")).toEqual({ kind: "unknown", command: "voar" });
  });
});
