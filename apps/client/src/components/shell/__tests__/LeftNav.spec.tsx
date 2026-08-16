import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { LeftNav } from "../LeftNav";

describe("LeftNav", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders the 5 nav entries", () => {
    render(<LeftNav />);

    expect(screen.getByText("Parc")).toBeInTheDocument();
    expect(screen.getByText("Personnel")).toBeInTheDocument();
    expect(screen.getByText("Finances")).toBeInTheDocument();
    expect(screen.getByText("Marché")).toBeInTheDocument();
    expect(screen.getByText("Monde")).toBeInTheDocument();
  });

  it("only enables the Parc entry", () => {
    render(<LeftNav />);

    expect(screen.getByRole("button", { name: /parc/i })).toBeEnabled();
    expect(screen.getByRole("button", { name: /personnel/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /finances/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /marché/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /monde/i })).toBeDisabled();
  });
});
