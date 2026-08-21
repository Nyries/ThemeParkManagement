import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Toolbar } from "../Toolbar";

describe("Toolbar", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders the 4 tool buttons", () => {
    render(<Toolbar mode={"selection"} onModeChange={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Terrain" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Chemin" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Bâtiment" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retirer" })).toBeInTheDocument();
  });

  it("marks the active tool as pressed", () => {
    render(<Toolbar mode="terrain" onModeChange={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Terrain" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Chemin" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("selects a tool when clicking it", async () => {
    const onModeChange = vi.fn();
    const user = userEvent.setup();

    render(<Toolbar mode={"selection"} onModeChange={onModeChange} />);
    await user.click(screen.getByRole("button", { name: "Bâtiment" }));

    expect(onModeChange).toHaveBeenCalledWith("building");
  });

  it("deselects the active tool when clicking it again", async () => {
    const onModeChange = vi.fn();
    const user = userEvent.setup();

    render(<Toolbar mode="remove" onModeChange={onModeChange} />);
    await user.click(screen.getByRole("button", { name: "Retirer" }));

    expect(onModeChange).toHaveBeenCalledWith("selection");
  });
});
