/**
 * Reusable "at your own risk" acknowledgement.
 *
 * Renders a warning banner plus a required checkbox. Lifted from the
 * pattern in FirmwareUpdateDialog.tsx so any high-risk enablement (firmware
 * update, AI assistant, etc.) can share the same UX and styling.
 *
 * Controlled component: parent owns the `acknowledged` state and gates the
 * action on `onAcknowledgedChange`. The `risk` prop drives color intensity
 * (high = red, medium = amber).
 */
import { CSSProperties, ReactNode } from 'react';
import './RiskAcknowledgement.css';

export type RiskLevel = 'high' | 'medium';

export interface RiskAcknowledgementProps {
  /** Current ack state (controlled). */
  acknowledged: boolean;
  /** Called when the checkbox toggles. */
  onAcknowledgedChange: (ack: boolean) => void;
  /** Risk intensity. `high` (default) = red, `medium` = amber. */
  risk?: RiskLevel;
  /** Short label, e.g. "High risk". */
  label?: string;
  /** The body text shown in the warning banner. */
  warning: ReactNode;
  /** The text next to the acknowledgement checkbox. */
  acknowledgementText: ReactNode;
  /** Disable the checkbox (e.g. while an operation is in progress). */
  disabled?: boolean;
  className?: string;
  style?: CSSProperties;
}

export function RiskAcknowledgement({
  acknowledged,
  onAcknowledgedChange,
  risk = 'high',
  label,
  warning,
  acknowledgementText,
  disabled = false,
  className,
  style,
}: RiskAcknowledgementProps) {
  const riskLabel =
    label ?? (risk === 'high' ? 'High risk' : 'Caution');
  return (
    <div
      className={`risk-ack risk-ack-${risk}${className ? ' ' + className : ''}`}
      style={style}
    >
      <div className="risk-ack-banner">
        <strong className="risk-ack-label">{riskLabel}</strong>
        <div className="risk-ack-warning">{warning}</div>
      </div>
      <label className="risk-ack-checkbox">
        <input
          type="checkbox"
          checked={acknowledged}
          onChange={(e) => onAcknowledgedChange(e.target.checked)}
          disabled={disabled}
        />
        <span>{acknowledgementText}</span>
      </label>
    </div>
  );
}
