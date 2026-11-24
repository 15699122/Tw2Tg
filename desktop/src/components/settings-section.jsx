import { useId } from "react";
import { Button } from "./ui/button";
import { CardDescription, CardTitle } from "./ui/card";
import Icon from "./icon.jsx";

export default function SettingsSection({
  id,
  title,
  description,
  icon,
  expanded,
  onToggle,
  actions,
  children,
  className = "",
}) {
  const contentId = useId();
  const titleId = useId();
  // A page whose only panel is this one passes no `onToggle`: collapsing it would
  // hide the whole page, so the panel renders as a static card without a chevron.
  const collapsible = typeof onToggle === "function";
  const expandedState = collapsible ? Boolean(expanded) : true;
  const iconNode = <span className={`component-icon settings-panel-icon ${icon === "telegram" ? "telegram-icon" : ""}`} aria-hidden="true"><Icon name={icon} size={20} /></span>;
  const copyNode = (
    <span className="settings-panel-copy">
      <CardTitle id={titleId}>{title}</CardTitle>
      <CardDescription>{description}</CardDescription>
    </span>
  );
  return (
    <section className={`settings-section settings-panel ${collapsible ? "" : "settings-panel-static"} ${className}`} id={id}>
      <div className="settings-panel-header">
        {collapsible ? (
          <Button
            type="button"
            className="settings-panel-toggle"
            variant="ghost"
            size="unstyled"
            aria-expanded={expandedState}
            aria-controls={contentId}
            aria-label={`${title}，${expandedState ? "收起" : "展开"}`}
            onClick={onToggle}
          >
            {iconNode}
            {copyNode}
            <span className="settings-panel-chevron" aria-hidden="true"><Icon name="chevron" size={18} /></span>
          </Button>
        ) : (
          <div className="settings-panel-static-header" role="group" aria-labelledby={titleId}>
            {iconNode}
            {copyNode}
          </div>
        )}
        {actions && <div className="settings-panel-actions">{actions}</div>}
      </div>
      <div className="settings-panel-content" id={contentId} hidden={!expandedState}>
        {children}
      </div>
    </section>
  );
}