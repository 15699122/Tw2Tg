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
  const expandedState = Boolean(expanded);
  return (
    <section className={`settings-section settings-panel ${className}`} id={id}>
      <div className="settings-panel-header">
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
          <span className={`component-icon settings-panel-icon ${icon === "telegram" ? "telegram-icon" : ""}`} aria-hidden="true"><Icon name={icon} size={20} /></span>
          <span className="settings-panel-copy">
            <CardTitle id={titleId}>{title}</CardTitle>
            <CardDescription>{description}</CardDescription>
          </span>
          <span className="settings-panel-chevron" aria-hidden="true"><Icon name="chevron" size={18} /></span>
        </Button>
        {actions && <div className="settings-panel-actions">{actions}</div>}
      </div>
      <div className="settings-panel-content" id={contentId} hidden={!expandedState}>
        {children}
      </div>
    </section>
  );
}