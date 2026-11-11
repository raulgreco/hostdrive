import { describe, expect, it } from "vitest";
import { agents } from "./workspace";

describe("agent registry seed", () => {
  it("contains each supported CLI exactly once", () => {
    expect(agents.map((agent) => agent.id)).toEqual(["codex", "claude", "opencode", "kimi"]);
    expect(new Set(agents.map((agent) => agent.id)).size).toBe(agents.length);
  });

  it("does not infer authentication from executable detection", () => {
    expect(agents.every((agent) => !("authenticated" in agent))).toBe(true);
  });
});
