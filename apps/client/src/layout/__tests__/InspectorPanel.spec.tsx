import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { InspectorPanel } from "../InspectorPanel";

describe("InspectorPanel", () => {
  afterEach(() => {
    cleanup();
  });

  it("shows the neutral state when nothing is selected", () => {
    render(<InspectorPanel selection={null} />);
    expect(screen.getByText("Aucune sélection")).toBeInTheDocument();
  });

  it("shows the selection label when something is selected", () => {
    render(
      <InspectorPanel selection={{ kind: "building", label: "Montagnes russes" }} />,
    );
    expect(screen.getByText("Montagnes russes")).toBeInTheDocument();
  });

  it("always renders the journal placeholder", () => {
    render(<InspectorPanel selection={null} />);
    expect(screen.getByText("Journal")).toBeInTheDocument();
  });
});
