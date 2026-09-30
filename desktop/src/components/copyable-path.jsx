import Icon from "./icon.jsx";
import { displayFileName } from "../lib/ui-state.js";

export default function CopyablePath({ label, value, copied, onCopy }) {
  const shortName = displayFileName(value);
  const displayValue = value || "未检测到路径";
  return (
    <div className="path-block">
      <span className="field-label">{label}</span>
      <button
        type="button"
        className="copyable-path"
        title={displayValue}
        aria-label={`${label}：${displayValue}，点击复制完整路径`}
        onClick={() => onCopy?.(value)}
      >
        <Icon name="folder" size={15} />
        <span className="copyable-path-text">
          <strong>{shortName}</strong>
          <small>{displayValue}</small>
        </span>
        {copied ? <span className="copyable-path-copied">已复制</span> : <Icon name="copy" size={14} />}
      </button>
    </div>
  );
}
