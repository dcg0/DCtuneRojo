/**
 * Shared UI primitives.
 *
 * Import from this barrel rather than reaching into individual files:
 *
 *   import { Dialog, Button, EmptyState, FormField } from '../common';
 *
 * These primitives are theme-token driven — do not hard-code colors in
 * consuming components, use the variables in `themes/variables.css`.
 */

export { Dialog } from './Dialog';
export type { DialogProps, DialogSize } from './Dialog';

export { Button } from './Button';
export type { ButtonProps, ButtonVariant, ButtonSize } from './Button';

export { EmptyState } from './EmptyState';
export type { EmptyStateProps } from './EmptyState';

export { FormField } from './FormField';
export type { FormFieldProps } from './FormField';

export { default as ErrorBoundary } from './ErrorBoundary';

export { RiskAcknowledgement } from './RiskAcknowledgement';
export type { RiskAcknowledgementProps, RiskLevel } from './RiskAcknowledgement';
