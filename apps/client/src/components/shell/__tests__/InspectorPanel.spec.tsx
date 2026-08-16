import { describe, it, expect, afterEach, vi } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { InspectorPanel } from "../InspectorPanel";
import type { ToolState } from "@/types/park/tool";

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

  it("shows the selection label when something is selected and no tool is active", () => {
    render(
      <InspectorPanel
        selection={{ kind: "building", label: "Montagnes russes" }}
        tool={NO_TOOL}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.getByText("Montagnes russes")).toBeInTheDocument();
  });

  it("never shows the selection while a construction tool is active", () => {
    render(
      <InspectorPanel
        selection={{ kind: "building", label: "Montagnes russes" }}
        tool={{ mode: "terrain" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.queryByText("Montagnes russes")).not.toBeInTheDocument();
  });

  it("shows the selection label while the remove tool is active", () => {
    render(
      <InspectorPanel
        selection={{ kind: "building", label: "Montagnes russes" }}
        tool={{ mode: "remove" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.getByText("Montagnes russes")).toBeInTheDocument();
  });

  it("renders the journal placeholder when no tool is active", () => {
    render(
      <InspectorPanel selection={null} tool={NO_TOOL} onToolChange={vi.fn()} />,
    );
    expect(screen.getByText("Journal")).toBeInTheDocument();
  });

  it("renders the journal placeholder while the remove tool is active", () => {
    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "remove" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.getByText("Journal")).toBeInTheDocument();
  });

  it("hides the journal placeholder while a construction tool is active", () => {
    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "terrain" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.queryByText("Journal")).not.toBeInTheDocument();
  });

  it("shows the material selector when the terrain tool is active, defaulting to grass", () => {
    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "terrain" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.getByText("Matériau")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Herbe" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Eau" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("reflects the selected material as pressed", () => {
    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "terrain", selectedMaterialId: "water" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.getByRole("button", { name: "Eau" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Herbe" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("selects a material when clicking it", async () => {
    const onToolChange = vi.fn();
    const user = userEvent.setup();

    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "terrain" }}
        onToolChange={onToolChange}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Eau" }));

    expect(onToolChange).toHaveBeenCalledWith({
      mode: "terrain",
      selectedMaterialId: "water",
    });
  });

  it("shows the building catalogue, grouped by category, when the building tool is active", () => {
    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "building" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(screen.getByText("Catalogue")).toBeInTheDocument();
    expect(screen.getByText("Commodités")).toBeInTheDocument();
    expect(screen.getByText("Attractions")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Sit-Down Restaurant" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Roller Coaster" }),
    ).toBeInTheDocument();
  });

  it("defaults the selected building to sit_down_restaurant, and reflects an explicit selection", () => {
    const { rerender } = render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "building" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Sit-Down Restaurant" }),
    ).toHaveAttribute("aria-pressed", "true");

    rerender(
      <InspectorPanel
        selection={null}
        tool={{ mode: "building", selectedBuildingId: "roller_coaster" }}
        onToolChange={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Roller Coaster" }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(
      screen.getByRole("button", { name: "Sit-Down Restaurant" }),
    ).toHaveAttribute("aria-pressed", "false");
  });

  it("selects a building when clicking it", async () => {
    const onToolChange = vi.fn();
    const user = userEvent.setup();

    render(
      <InspectorPanel
        selection={null}
        tool={{ mode: "building" }}
        onToolChange={onToolChange}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Ferris Wheel" }));

    expect(onToolChange).toHaveBeenCalledWith({
      mode: "building",
      selectedBuildingId: "ferris_wheel",
    });
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
