import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/react";
import { SecondaryToolbar } from "../SecondaryToolbar";

describe("SecondaryToolbar", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders nothing when no tool is active", () => {
    const { container } = render(
      <SecondaryToolbar mode={null} onRotate={vi.fn()} />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("renders nothing for tools without contextual controls", () => {
    const { container } = render(
      <SecondaryToolbar mode="terrain" onRotate={vi.fn()} />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("shows both rotate buttons when the building tool is active", () => {
    render(<SecondaryToolbar mode="building" onRotate={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Rotation (R)" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Rotation inversée (Maj+R)" }),
    ).toBeInTheDocument();
  });

  it("rotates clockwise when clicking the clockwise button", () => {
    const onRotate = vi.fn();
    render(<SecondaryToolbar mode="building" onRotate={onRotate} />);

    fireEvent.click(screen.getByRole("button", { name: "Rotation (R)" }));

    expect(onRotate).toHaveBeenCalledWith(false);
  });

  it("rotates counter-clockwise when clicking the counter-clockwise button", () => {
    const onRotate = vi.fn();
    render(<SecondaryToolbar mode="building" onRotate={onRotate} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Rotation inversée (Maj+R)" }),
    );

    expect(onRotate).toHaveBeenCalledWith(true);
  });
});
