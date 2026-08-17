/**
 * Automated AFR transport-delay measurement.
 *
 * Applies a timed enrichment step so the wideband's response can be measured
 * from the log. The operator holds the engine at an operating point; this only
 * touches fuelling, and only ever enriches.
 *
 * The safety story is deliberately visible in the UI rather than buried: the
 * dialog states that the step is rich-only, RAM-only and never burned, shows
 * the exact values before anything is applied, and keeps a large abort control
 * available for the whole run.
 */
import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { AlertTriangle, Play, Square } from 'lucide-react';
import './AfrDelayTestDialog.css';

interface Progress {
  phase: string;
  step: number;
  totalSteps: number;
  appliedValue: number;
  baselineValue: number;
  message: string;
}

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

export const AfrDelayTestDialog: React.FC<Props> = ({ isOpen, onClose }) => {
  const [stepPercent, setStepPercent] = useState(8);
  const [holdMs, setHoldMs] = useState(2000);
  const [settleMs, setSettleMs] = useState(3000);
  const [repeats, setRepeats] = useState(5);

  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const un = listen<Progress>('afr_delay_test:progress', (e) => setProgress(e.payload));
    return () => { un.then((f) => f()); };
  }, []);

  const start = useCallback(async () => {
    setError(null);
    setResult(null);
    setRunning(true);
    try {
      const summary = await invoke<string>('run_afr_delay_test', {
        stepPercent, holdMs, settleMs, repeats,
      });
      setResult(summary);
    } catch (e) {
      // A failure here may mean the engine is still enriched, so the message
      // from the backend (which says whether the restore succeeded) matters —
      // show it verbatim rather than summarising.
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }, [stepPercent, holdMs, settleMs, repeats]);

  const abort = useCallback(async () => {
    try { await invoke('abort_afr_delay_test'); } catch { /* best effort */ }
  }, []);

  if (!isOpen) return null;

  const estSeconds = Math.round((repeats * (holdMs + settleMs)) / 1000);

  return (
    <div className="dialog-overlay" onClick={running ? undefined : onClose}>
      <div className="afr-delay-dialog" onClick={(e) => e.stopPropagation()}>
        <h2>AFR Delay Test</h2>

        <p className="afr-delay-intro">
          Applies a timed enrichment step so the delay between a fuelling change and
          the wideband seeing it can be measured from the log. Repeat at several
          rpm and load points to build a delay map.
        </p>

        <div className="afr-delay-safety">
          <AlertTriangle size={16} />
          <div>
            <strong>Enrichment only, RAM only, never burned.</strong>
            <ul>
              <li>The step can only make the mixture richer, never leaner.</li>
              <li>Written to live memory — cycling the key restores the stored tune.</li>
              <li><code>reqFuel</code> is restored after every step and on abort.</li>
              <li>Hold the engine steady yourself; this does not touch throttle.</li>
            </ul>
          </div>
        </div>

        <div className="afr-delay-fields">
          <label>
            Step size
            <input type="number" min={3} max={20} step={1} value={stepPercent}
              disabled={running}
              onChange={(e) => setStepPercent(Number(e.target.value))} />
            <span className="unit">% richer</span>
          </label>
          <label>
            Hold
            <input type="number" min={500} max={5000} step={100} value={holdMs}
              disabled={running}
              onChange={(e) => setHoldMs(Number(e.target.value))} />
            <span className="unit">ms</span>
          </label>
          <label>
            Settle
            <input type="number" min={500} max={10000} step={100} value={settleMs}
              disabled={running}
              onChange={(e) => setSettleMs(Number(e.target.value))} />
            <span className="unit">ms</span>
          </label>
          <label>
            Repeats
            <input type="number" min={1} max={20} step={1} value={repeats}
              disabled={running}
              onChange={(e) => setRepeats(Number(e.target.value))} />
            <span className="unit">steps</span>
          </label>
        </div>

        <p className="afr-delay-estimate">
          Approximately {estSeconds}s. Start recording first, and keep rpm and load
          steady throughout.
        </p>

        {progress && (
          <div className={`afr-delay-progress phase-${progress.phase}`}>
            <div className="afr-delay-bar">
              <div style={{ width: `${(progress.step / Math.max(progress.totalSteps, 1)) * 100}%` }} />
            </div>
            <div className="afr-delay-status">
              <span className="phase">{progress.phase}</span>
              <span>{progress.message}</span>
            </div>
            {progress.baselineValue > 0 && (
              <div className="afr-delay-values">
                reqFuel baseline {progress.baselineValue.toFixed(1)} ms
                {progress.phase === 'enriching' && ` -> applying ${progress.appliedValue.toFixed(1)} ms`}
              </div>
            )}
          </div>
        )}

        {result && <div className="afr-delay-result">{result}</div>}
        {error && (
          <div className="afr-delay-error">
            <AlertTriangle size={16} />
            <span>{error}</span>
          </div>
        )}

        <div className="afr-delay-actions">
          {running ? (
            <button type="button" className="danger" onClick={abort}>
              <Square size={14} /> Abort and restore
            </button>
          ) : (
            <>
              <button type="button" onClick={onClose}>Close</button>
              <button type="button" className="primary" onClick={start}>
                <Play size={14} /> Start test
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
};

export default AfrDelayTestDialog;
