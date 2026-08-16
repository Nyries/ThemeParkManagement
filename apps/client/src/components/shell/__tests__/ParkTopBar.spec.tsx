import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { ParkTopBar } from "../ParkTopBar";

describe("ParkTopBar", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders the current park name", () => {
    render(<ParkTopBar balance={0} visitorCount={0} />);
    expect(screen.getByText("Prairie Meadows")).toBeInTheDocument();
  });

  it("renders the balance and visitor count", () => {
    render(<ParkTopBar balance={1500} visitorCount={3} />);
    expect(screen.getByText("1 500 €")).toBeInTheDocument();
    expect(screen.getByText("3 visiteurs")).toBeInTheDocument();
  });

  it("uses the singular form for a single visitor", () => {
    render(<ParkTopBar balance={0} visitorCount={1} />);
    expect(screen.getByText("1 visiteur")).toBeInTheDocument();
  });
});
