import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { RotateCcw, RotateCw } from "lucide-react";
import { Button } from "./ui/button";
import type { ToolMode } from "@/park/tool";

interface SecondaryToolbarProps {
  mode: ToolMode;
  onRotate: (reverse: boolean) => void;
}

export interface SecondaryToolbarHandle {
  triggerRotate: (reverse: boolean) => void;
}

type PressedButton = "cw" | "ccw" | null;

// How long a button stays visually "pressed" when triggered from the
// keyboard — matches the Button's own transition duration so the fake press
// reads the same as a real click.
const PRESS_DURATION_MS = 150;

// Reproduces the ghost variant's :hover styling (bg-muted/text-foreground)
// plus the :active translate — neither pseudo-class fires for a
// programmatic trigger, so this is applied manually while "pressed".
const PRESSED_VISUAL_CLASSES =
  "bg-muted text-foreground dark:bg-muted/50 translate-y-px";

// Contextual controls that only make sense for the currently active tool —
// kept out of the main Toolbar so it stays a fixed, predictable set of
// 4 buttons. Renders nothing when the active tool has no extra controls.
// Rendered as a sibling right below the main Toolbar (see Park.tsx), inside
// a shared centered flex column, so it visually emerges out of it.
//
// The R/Shift+R keyboard shortcut (Park.tsx) calls triggerRotate through
// this imperative handle instead of duplicating the rotation logic. Neither
// a real click's :active feedback nor :hover ever fire for a programmatic
// trigger, so this also toggles a manual "pressed" class on the matching
// button to reproduce that feedback.
export const SecondaryToolbar = forwardRef<
  SecondaryToolbarHandle,
  SecondaryToolbarProps
>(function SecondaryToolbar({ mode, onRotate }, ref) {
  const [pressed, setPressed] = useState<PressedButton>(null);
  const pressTimeoutRef = useRef<number | undefined>(undefined);

  useImperativeHandle(ref, () => ({
    triggerRotate(reverse: boolean) {
      onRotate(reverse);
      setPressed(reverse ? "ccw" : "cw");
      window.clearTimeout(pressTimeoutRef.current);
      pressTimeoutRef.current = window.setTimeout(() => {
        setPressed(null);
      }, PRESS_DURATION_MS);
    },
  }));

  useEffect(() => {
    return () => window.clearTimeout(pressTimeoutRef.current);
  }, []);

  if (mode !== "building") {
    return null;
  }

  return (
    <div className="flex gap-1 rounded-lg border border-border bg-white/80 p-1 shadow-sm backdrop-blur-sm transition-all duration-150 ease-out starting:-translate-y-2 starting:opacity-0">
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label="Tourner (R)"
        className={pressed === "cw" ? PRESSED_VISUAL_CLASSES : undefined}
        onClick={() => onRotate(false)}
      >
        <RotateCw className="size-4" />
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label="Tourner dans l'autre sens (Maj+R)"
        className={pressed === "ccw" ? PRESSED_VISUAL_CLASSES : undefined}
        onClick={() => onRotate(true)}
      >
        <RotateCcw className="size-4" />
      </Button>
    </div>
  );
});
