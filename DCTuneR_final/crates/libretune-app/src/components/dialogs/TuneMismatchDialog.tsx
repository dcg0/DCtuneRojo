import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Dialog, Button } from '../common';
import './TuneMismatchDialog.css';

export interface TuneMismatchInfo {
  ecu_pages: number[];
  project_pages: number[];
  diff_pages: number[];
}

interface TuneMismatchDialogProps {
  isOpen: boolean;
  mismatchInfo: TuneMismatchInfo | null;
  onClose: () => void;
  onUseProject: () => void;
  onUseECU: () => void;
}

export default function TuneMismatchDialog({
  isOpen,
  mismatchInfo,
  onClose,
  onUseProject,
  onUseECU,
}: TuneMismatchDialogProps) {
  const [isLoading, setIsLoading] = useState(false);

  if (!isOpen || !mismatchInfo) return null;

  const handleUseProject = async () => {
    setIsLoading(true);
    try {
      await invoke('use_project_tune');
      onUseProject();
      onClose();
    } catch (err) {
      console.error('Failed to load project tune:', err);
      alert(`Failed to load project tune: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleUseECU = async () => {
    setIsLoading(true);
    try {
      await invoke('use_ecu_tune');
      onUseECU();
      onClose();
    } catch (err) {
      console.error('Failed to use ECU tune:', err);
      alert(`Failed to use ECU tune: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Dialog
      open={isOpen}
      onClose={onClose}
      title="Tune Mismatch Detected"
      size="md"
      className="tune-mismatch-dialog"
      closeOnBackdrop={!isLoading}
      closeOnEscape={!isLoading}
    >
      <Dialog.Body>
        <div className="tune-mismatch-warning">
          <p>
            <strong>The tune on the ECU differs from the tune in your project.</strong>
          </p>
          <p>
            The ECU has {mismatchInfo.ecu_pages.length} page(s) loaded, while your project has{' '}
            {mismatchInfo.project_pages.length} page(s).
            {mismatchInfo.diff_pages.length > 0 && (
              <> {mismatchInfo.diff_pages.length} page(s) have differences.</>
            )}
          </p>
        </div>

        <div className="tune-mismatch-options">
          <div className="tune-option">
            <h3>Use Project Tune</h3>
            <p>
              Load the tune from your project file. This will overwrite the ECU tune with your
              saved project data.
            </p>
            <Button variant="primary" onClick={handleUseProject} disabled={isLoading}>
              {isLoading ? 'Loading...' : 'Use Project Tune'}
            </Button>
          </div>

          <div className="tune-option">
            <h3>Use ECU Tune</h3>
            <p>
              Keep the tune currently on the ECU. Your project will be updated to match the ECU.
            </p>
            <Button variant="secondary" onClick={handleUseECU} disabled={isLoading}>
              {isLoading ? 'Loading...' : 'Use ECU Tune'}
            </Button>
          </div>
        </div>
      </Dialog.Body>

      <Dialog.Footer>
        <Button variant="secondary" onClick={onClose} disabled={isLoading}>
          Cancel
        </Button>
      </Dialog.Footer>
    </Dialog>
  );
}
