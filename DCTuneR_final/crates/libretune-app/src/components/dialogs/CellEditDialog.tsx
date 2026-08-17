import React, { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Dialog, Button } from '../common';
import './CellEditDialog.css';

export interface CellEditDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onApply: (value: number) => void;
  currentValue: number;
  cellRow: number;
  cellCol: number;
  xBinValue: number;
  yBinValue: number;
  xAxisName: string;
  yAxisName: string;
  units?: string;
  minValue?: number;
  maxValue?: number;
  decimals?: number;
}

export default function CellEditDialog({
  isOpen,
  onClose,
  onApply,
  currentValue,
  cellRow,
  cellCol,
  xBinValue,
  yBinValue,
  xAxisName,
  yAxisName,
  units = '',
  minValue,
  maxValue,
  decimals = 2,
}: CellEditDialogProps) {
  const { t } = useTranslation(['dialog', 'common']);
  const [inputValue, setInputValue] = useState('');
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      setInputValue(currentValue.toFixed(decimals));
      setError(null);
      setTimeout(() => inputRef.current?.select(), 50);
    }
  }, [isOpen, currentValue, decimals]);

  const validate = (value: string): string | null => {
    const num = parseFloat(value);
    if (isNaN(num)) return t('cellEdit.validNumber', { ns: 'dialog' });
    if (minValue !== undefined && num < minValue)
      return t('cellEdit.valueAtLeast', { ns: 'dialog', min: minValue });
    if (maxValue !== undefined && num > maxValue)
      return t('cellEdit.valueAtMost', { ns: 'dialog', max: maxValue });
    return null;
  };

  const handleInputChange = (value: string) => {
    setInputValue(value);
    setError(validate(value));
  };

  const handleApply = () => {
    const validationError = validate(inputValue);
    if (validationError) {
      setError(validationError);
      return;
    }
    onApply(parseFloat(inputValue));
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleApply();
  };

  const handleIncrement = (amount: number) => {
    const current = parseFloat(inputValue) || 0;
    const newValue = current + amount;
    setInputValue(newValue.toFixed(decimals));
    setError(validate(newValue.toString()));
  };

  return (
    <Dialog
      open={isOpen}
      onClose={onClose}
      size="sm"
      title={t('cellEdit.title', { ns: 'dialog' })}
      titleAdornment={
        <span className="cell-location">
          [{cellCol}, {cellRow}]
        </span>
      }
    >
      <Dialog.Body>
        <div className="cell-edit-info">
          <div className="info-row">
            <span className="info-label">{xAxisName}:</span>
            <span className="info-value">{xBinValue}</span>
          </div>
          <div className="info-row">
            <span className="info-label">{yAxisName}:</span>
            <span className="info-value">{yBinValue}</span>
          </div>
        </div>

        <div className="cell-edit-input-section">
          <div className="input-with-buttons">
            <button
              className="adjust-btn"
              onClick={() => handleIncrement(-1)}
              title={t('cellEdit.decreaseBy1', { ns: 'dialog' })}
            >
              −1
            </button>
            <button
              className="adjust-btn"
              onClick={() => handleIncrement(-0.1)}
              title={t('cellEdit.decreaseByPoint1', { ns: 'dialog' })}
            >
              −.1
            </button>
            <input
              ref={inputRef}
              type="number"
              value={inputValue}
              onChange={e => handleInputChange(e.target.value)}
              onKeyDown={handleKeyDown}
              step="any"
              className={`cell-value-input ${error ? 'error' : ''}`}
            />
            <button
              className="adjust-btn"
              onClick={() => handleIncrement(0.1)}
              title={t('cellEdit.increaseByPoint1', { ns: 'dialog' })}
            >
              +.1
            </button>
            <button
              className="adjust-btn"
              onClick={() => handleIncrement(1)}
              title={t('cellEdit.increaseBy1', { ns: 'dialog' })}
            >
              +1
            </button>
          </div>
          {units && <span className="units-label">{units}</span>}
        </div>

        {error && <div className="cell-edit-error">{error}</div>}

        <div className="cell-edit-range">
          {minValue !== undefined && (
            <span>
              {t('labels.min', { ns: 'common' })}: {minValue}
            </span>
          )}
          {maxValue !== undefined && (
            <span>
              {t('labels.max', { ns: 'common' })}: {maxValue}
            </span>
          )}
        </div>
      </Dialog.Body>

      <Dialog.Footer>
        <Button variant="secondary" onClick={onClose}>
          {t('actions.cancel', { ns: 'common' })}
        </Button>
        <Button variant="primary" onClick={handleApply} disabled={!!error}>
          {t('actions.apply', { ns: 'common' })}
        </Button>
      </Dialog.Footer>
    </Dialog>
  );
}
