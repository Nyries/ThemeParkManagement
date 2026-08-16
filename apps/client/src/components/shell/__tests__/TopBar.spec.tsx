import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { TopBar } from "../TopBar";

describe("TopBar", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders the game name", () => {
    render(<TopBar />);
    expect(screen.getByText("Park Horizon")).toBeInTheDocument();
  });

  it("renders a disabled search placeholder", () => {
    render(<TopBar />);
    expect(
      screen.getByRole("button", { name: /rechercher/i }),
    ).toBeDisabled();
  });

  it("renders an account avatar", () => {
    render(<TopBar />);
    expect(screen.getByText("C")).toBeInTheDocument();
  });
});
