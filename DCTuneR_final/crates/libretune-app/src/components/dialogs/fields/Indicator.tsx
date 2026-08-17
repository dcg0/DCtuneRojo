import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DialogComponent } from '../types';

/// Renders a single boolean indicator (light + label) by evaluating
/// `comp.expression` against the supplied channel-value `context`.
export function Indicator({
  comp,
  context,
}: {
  comp: DialogComponent;
  context: Record<string, number>;
}) {
  const [isOn, setIsOn] = useState(false);

  useEffect(() => {
    if (comp.expression) {
      invoke<boolean>('evaluate_expression', { expression: comp.expression, context })
        .then(setIsOn)
        .catch(console.error);
    }
  }, [comp.expression, context]);

  return (
    <div className="indicator-field">
      <div className={`indicator-light ${isOn ? 'on' : 'off'}`} />
      <span className="indicator-label">{isOn ? comp.label_on : comp.label_off}</span>
    </div>
  );
}
