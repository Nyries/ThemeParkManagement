import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { ParkTopBar } from "../ParkTopBar";

describe("ParkTopBar", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders the current park name", () => {
    render(<ParkTopBar />);
    expect(screen.getByText("Prairie Meadows")).toBeInTheDocument();
  });
});
