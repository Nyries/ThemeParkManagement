import { useEffect, type RefObject } from "react";
import type { SecondaryToolbarHandle } from "@/components/park/SecondaryToolbar";
import type { ToolState } from "@/types/park/tool";

// Keyboard shortcuts: 1-4 select a tool, R/Shift+R rotate the building
// ghost, Escape cancels the active tool. All of them must preempt the
// browser's own default behavior for that key.
//
// Matched on event.code (physical key position) rather than event.key: on
// an AZERTY layout the digit row only produces "1".."4" while holding
// Shift, so event.key for an unshifted press is "&"/"é"/'"'/"'" instead —
// event.code stays "Digit1".."Digit4" regardless of layout or Shift state.
const SHORTCUT_CODES = new Set([
  "Digit1",
  "Digit2",
  "Digit3",
  "Digit4",
  "KeyR",
  "Escape",
]);

interface UseParkKeyboardShortcutsOptions {
  containerRef: RefObject<HTMLDivElement | null>;
  toolRef: RefObject<ToolState>;
  onToolChange: (tool: ToolState) => void;
  rotateButtonRef: RefObject<SecondaryToolbarHandle | null>;
}

// Listens on the Park's canvas container (not document/window) so the
// container's own focus-capture (tabIndex + focus() on mount, see Park.tsx)
// is what determines whether these shortcuts fire — avoids stealing keyboard
// input from the rest of the app.
export function useParkKeyboardShortcuts({
  containerRef,
  toolRef,
  onToolChange,
  rotateButtonRef,
}: UseParkKeyboardShortcutsOptions) {
  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (!SHORTCUT_CODES.has(event.code)) {
        return;
      }
      event.preventDefault();

      const currentTool = toolRef.current;
      switch (event.code) {
        case "Digit1":
          onToolChange({
            ...currentTool,
            mode: currentTool.mode === "terrain" ? null : "terrain",
          });
          break;
        case "Digit2":
          onToolChange({
            ...currentTool,
            mode:
              currentTool.mode === "infrastructure" ? null : "infrastructure",
          });
          break;
        case "Digit3":
          onToolChange({
            ...currentTool,
            mode: currentTool.mode === "building" ? null : "building",
          });
          break;
        case "Digit4":
          onToolChange({
            ...currentTool,
            mode: currentTool.mode === "remove" ? null : "remove",
          });
          break;
        case "KeyR":
          if (currentTool.mode === "building") {
            // Goes through the rotate button's own handler (and its visual
            // "pressed" feedback) rather than updating the tool state
            // directly, so the shortcut behaves like clicking the button.
            rotateButtonRef.current?.triggerRotate(event.shiftKey);
          }
          break;
        case "Escape":
          onToolChange({ ...currentTool, mode: null });
          break;
        default:
          break;
      }
    }

    container.addEventListener("keydown", handleKeyDown);
    return () => container.removeEventListener("keydown", handleKeyDown);
  }, [containerRef, toolRef, onToolChange, rotateButtonRef]);
}
