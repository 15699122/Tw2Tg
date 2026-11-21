import { useEffect, useRef } from "react";

/** A labelled boolean control; never submits its containing form. */
export function Toggle({ checked, onCheckedChange, children, className = "", disabled = false, restoreFocusAfterBusy = false }) {
  const inputRef = useRef(null);
  const restoreFocus = useRef(false);
  useEffect(() => {
    if (!disabled && restoreFocus.current) {
      if (document.activeElement === document.body) inputRef.current?.focus();
      restoreFocus.current = false;
    }
  }, [disabled]);
  const change = (value) => {
    if (restoreFocusAfterBusy && document.activeElement === inputRef.current) restoreFocus.current = true;
    onCheckedChange(value);
  };
  return <label className={`ui-toggle-field ${className}`}>
    <input ref={inputRef} className="ui-toggle-input" type="checkbox" role="switch" checked={checked} disabled={disabled} onChange={(event) => change(event.target.checked)} onKeyDown={(event) => {
      if (event.key !== "Enter") return;
      event.preventDefault();
      if (!disabled && !event.repeat) change(!checked);
    }} />
    <span>{children}</span>
  </label>;
}
