import { describe, it, expect, afterEach, vi } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { InspectorPanel } from "../InspectorPanel";
import type { ToolState } from "@/park/tool";

const NO_TOOL: ToolState = { mode: null };

describe("InspectorPanel", () => {
  afterEach(() => {
    cleanup();
  });

  it("shows the neutral state when nothing is selected and no tool is active", () => {
    render(
      <InspectorPanel selection={null} tool={NO_TOOL} onToolChange={vi.fn()} />,
    );
    expect(screen.getByText("Aucune sélection")).toBeInTheDocument();
  });

  it("shows the selection label when something is selected", () => {
    render(
      <InspectorPanel
        selection={{ kind: "building", label: "Montagnes russes" }}
        tool={NO_TOOL}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.getByText("Montagnes russes")).toBeInTheDocument();
  });

  it("always renders the journal placeholder", () => {
    render(
      <InspectorPanel selection={null} tool={NO_TOOL} onToolChange={vi.fn()} />,
    );
    expect(screen.getByText("Journal")).toBeInTheDocument();
  });

  it("shows the terrain placeholder when the terrain tool is active", () => {
    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "terrain" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.getByText("Sélection de matériau — à venir")).toBeInTheDocument();
  });

  it("shows the catalog placeholder when the building tool is active", () => {
    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "building" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(
      screen.getByText("Catalogue de bâtiments — à venir"),
    ).toBeInTheDocument();
  });

  it("shows instructions when the infrastructure tool is active", () => {
    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "infrastructure" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(
      screen.getByText("Tracer un chemin sur le canvas"),
    ).toBeInTheDocument();
  });

  it("shows instructions when the remove tool is active", () => {
    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "remove" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(
      screen.getByText("Cliquer un élément à retirer"),
    ).toBeInTheDocument();
  });
});
