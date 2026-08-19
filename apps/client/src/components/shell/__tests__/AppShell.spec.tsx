import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { AppShell } from "../AppShell";

describe("AppShell", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders the nav, the top bar and the children", () => {
    render(
      <AppShell>
        <div>Park canvas</div>
      </AppShell>,
    );

    expect(screen.getByText("Park Horizon")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /parc/i })).toBeInTheDocument();
    expect(screen.getByText("Park canvas")).toBeInTheDocument();
  });
});
