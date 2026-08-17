import { useState, useEffect } from 'react';
import { Plus, Minus, RotateCcw } from 'lucide-react';
import { Dialog, Button } from '../common';
import './RebinDialog.css';

export interface RebinDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onApply: (newXBins: number[], newYBins: number[], interpolate: boolean) => void;
  currentXBins: number[];
  currentYBins: number[];
  xAxisName: string;
  yAxisName: string;
}

export default function RebinDialog({
  isOpen,
  onClose,
  onApply,
  currentXBins,
  currentYBins,
  xAxisName,
  yAxisName,
}: RebinDialogProps) {
  const [newXBins, setNewXBins] = useState<number[]>([]);
  const [newYBins, setNewYBins] = useState<number[]>([]);
  const [interpolateZ, setInterpolateZ] = useState(true);

  useEffect(() => {
    if (isOpen) {
      setNewXBins([...currentXBins]);
      setNewYBins([...currentYBins]);
      setInterpolateZ(true);
    }
  }, [isOpen, currentXBins, currentYBins]);

  const handleXBinChange = (index: number, value: string) => {
    const numValue = parseFloat(value);
    if (!isNaN(numValue)) {
      const updated = [...newXBins];
      updated[index] = numValue;
      setNewXBins(updated);
    }
  };

  const handleYBinChange = (index: number, value: string) => {
    const numValue = parseFloat(value);
    if (!isNaN(numValue)) {
      const updated = [...newYBins];
      updated[index] = numValue;
      setNewYBins(updated);
    }
  };

  const addXBin = () => {
    const lastValue = newXBins[newXBins.length - 1] || 0;
    const step =
      newXBins.length > 1
        ? newXBins[newXBins.length - 1] - newXBins[newXBins.length - 2]
        : 100;
    setNewXBins([...newXBins, lastValue + step]);
  };

  const removeXBin = () => {
    if (newXBins.length > 1) setNewXBins(newXBins.slice(0, -1));
  };

  const addYBin = () => {
    const lastValue = newYBins[newYBins.length - 1] || 0;
    const step =
      newYBins.length > 1
        ? newYBins[newYBins.length - 1] - newYBins[newYBins.length - 2]
        : 10;
    setNewYBins([...newYBins, lastValue + step]);
  };

  const removeYBin = () => {
    if (newYBins.length > 1) setNewYBins(newYBins.slice(0, -1));
  };

  const resetToOriginal = () => {
    setNewXBins([...currentXBins]);
    setNewYBins([...currentYBins]);
  };

  const generateLinearBins = (count: number, min: number, max: number): number[] => {
    const bins: number[] = [];
    const step = (max - min) / (count - 1);
    for (let i = 0; i < count; i++) {
      bins.push(Math.round((min + step * i) * 100) / 100);
    }
    return bins;
  };

  const handleGenerateX = () => {
    const min = Math.min(...currentXBins);
    const max = Math.max(...currentXBins);
    setNewXBins(generateLinearBins(newXBins.length, min, max));
  };

  const handleGenerateY = () => {
    const min = Math.min(...currentYBins);
    const max = Math.max(...currentYBins);
    setNewYBins(generateLinearBins(newYBins.length, min, max));
  };

  const handleApply = () => {
    const sortedX = [...newXBins].sort((a, b) => a - b);
    const sortedY = [...newYBins].sort((a, b) => a - b);
    onApply(sortedX, sortedY, interpolateZ);
    onClose();
  };

  const titleAdornment = (
    <button
      type="button"
      className="rebin-reset-btn"
      onClick={resetToOriginal}
      title="Reset to original bins"
      aria-label="Reset to original bins"
    >
      <RotateCcw size={16} />
    </button>
  );

  return (
    <Dialog
      open={isOpen}
      onClose={onClose}
      title="Re-bin Table"
      titleAdornment={titleAdornment}
      size="lg"
      className="rebin-dialog"
    >
      <Dialog.Body className="rebin-dialog-content">
        {/* X Axis Section */}
        <section className="rebin-section">
          <div className="rebin-section-header">
            <h3>
              {xAxisName} Bins ({newXBins.length})
            </h3>
            <div className="rebin-section-actions">
              <button
                type="button"
                className="rebin-icon-btn"
                onClick={handleGenerateX}
                title="Generate linear spacing"
              >
                Linear
              </button>
              <button
                type="button"
                className="rebin-icon-btn"
                onClick={removeXBin}
                disabled={newXBins.length <= 1}
                aria-label="Remove last X bin"
              >
                <Minus size={14} />
              </button>
              <button
                type="button"
                className="rebin-icon-btn"
                onClick={addXBin}
                aria-label="Add X bin"
              >
                <Plus size={14} />
              </button>
            </div>
          </div>
          <div className="rebin-bins-grid">
            {newXBins.map((val, i) => (
              <input
                key={`x-${i}`}
                type="number"
                value={val}
                step="any"
                onChange={(e) => handleXBinChange(i, e.target.value)}
                className="rebin-bin-input"
              />
            ))}
          </div>
        </section>

        {/* Y Axis Section */}
        <section className="rebin-section">
          <div className="rebin-section-header">
            <h3>
              {yAxisName} Bins ({newYBins.length})
            </h3>
            <div className="rebin-section-actions">
              <button
                type="button"
                className="rebin-icon-btn"
                onClick={handleGenerateY}
                title="Generate linear spacing"
              >
                Linear
              </button>
              <button
                type="button"
                className="rebin-icon-btn"
                onClick={removeYBin}
                disabled={newYBins.length <= 1}
                aria-label="Remove last Y bin"
              >
                <Minus size={14} />
              </button>
              <button
                type="button"
                className="rebin-icon-btn"
                onClick={addYBin}
                aria-label="Add Y bin"
              >
                <Plus size={14} />
              </button>
            </div>
          </div>
          <div className="rebin-bins-grid">
            {newYBins.map((val, i) => (
              <input
                key={`y-${i}`}
                type="number"
                value={val}
                step="any"
                onChange={(e) => handleYBinChange(i, e.target.value)}
                className="rebin-bin-input"
              />
            ))}
          </div>
        </section>

        {/* Options */}
        <div className="rebin-options">
          <label className="rebin-checkbox-label">
            <input
              type="checkbox"
              checked={interpolateZ}
              onChange={(e) => setInterpolateZ(e.target.checked)}
            />
            Interpolate Z values (recommended)
          </label>
          <p className="rebin-option-hint">
            When enabled, existing values will be bilinearly interpolated to the new bin
            locations. When disabled, new cells will be initialized to zero.
          </p>
        </div>

        {/* Preview Info */}
        <div className="rebin-preview">
          <div className="rebin-preview-item">
            <span className="rebin-preview-label">Original Size:</span>
            <span className="rebin-preview-value">
              {currentXBins.length} × {currentYBins.length}
            </span>
          </div>
          <div className="rebin-preview-item">
            <span className="rebin-preview-label">New Size:</span>
            <span className="rebin-preview-value">
              {newXBins.length} × {newYBins.length}
            </span>
          </div>
        </div>
      </Dialog.Body>

      <Dialog.Footer>
        <Button variant="secondary" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="primary" onClick={handleApply}>
          Apply Re-bin
        </Button>
      </Dialog.Footer>
    </Dialog>
  );
}
