/**
 * Shared EmptyState primitive
 *
 * Standardized "nothing here yet" / "no results" panel used across views.
 * Replaces inconsistent empty-state markup in tables, dashboards, plugins,
 * project lists, etc.
 */

import React from 'react';
import './EmptyState.css';

export interface EmptyStateProps {
  /** Optional decorative icon (recommended: lucide icon at size 32-40). */
  icon?: React.ReactNode;
  /** Headline. */
  title: React.ReactNode;
  /** Supporting description. */
  description?: React.ReactNode;
  /** Optional primary action (e.g. <Button>Create Project</Button>). */
  action?: React.ReactNode;
  /** Optional secondary action. */
  secondaryAction?: React.ReactNode;
  /** Vertical padding scale. */
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export function EmptyState({
  icon,
  title,
  description,
  action,
  secondaryAction,
  size = 'md',
  className,
}: EmptyStateProps) {
  return (
    <div className={`lt-empty lt-empty--${size}${className ? ` ${className}` : ''}`}>
      {icon && <div className="lt-empty__icon">{icon}</div>}
      <div className="lt-empty__title">{title}</div>
      {description && <div className="lt-empty__desc">{description}</div>}
      {(action || secondaryAction) && (
        <div className="lt-empty__actions">
          {action}
          {secondaryAction}
        </div>
      )}
    </div>
  );
}

export default EmptyState;
