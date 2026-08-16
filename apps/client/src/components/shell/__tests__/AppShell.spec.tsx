import { describe, it, expect, afterEach, vi } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { AppShell } from "../AppShell";

const NO_TOOL = { mode: null } as const;

describe("AppShell", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders the nav, the top bars, the inspector and the children", () => {
    render(
      <AppShell selection={null} tool={NO_TOOL} onToolChange={vi.fn()}>
        <div>Park canvas</div>
      </AppShell>,
    );

    expect(screen.getByText("Park Horizon")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /parc/i })).toBeInTheDocument();
    expect(screen.getByText("Aucune sélection")).toBeInTheDocument();
    expect(screen.getByText("Park canvas")).toBeInTheDocument();
  });

  it("forwards the current selection to the inspector panel", () => {
    render(
      <AppShell
        selection={{ kind: "employee", label: "Agent d'entretien" }}
        tool={NO_TOOL}
        onToolChange={vi.fn()}
      >
        <div>Park canvas</div>
      </AppShell>,
    );

    expect(screen.getByText("Agent d'entretien")).toBeInTheDocument();
  });
});
