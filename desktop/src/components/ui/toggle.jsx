/** A labelled boolean control; never submits its containing form. */
export function Toggle({ checked, onCheckedChange, children, className = "", disabled = false }) {
  return <label className={`ui-toggle-field ${className}`}>
    <input className="ui-toggle-input" type="checkbox" role="switch" checked={checked} disabled={disabled} onChange={(event) => onCheckedChange(event.target.checked)} onKeyDown={(event) => {
      if (event.key !== "Enter") return;
      event.preventDefault();
      if (!disabled && !event.repeat) onCheckedChange(!checked);
    }} />
    <span>{children}</span>
  </label>;
}
