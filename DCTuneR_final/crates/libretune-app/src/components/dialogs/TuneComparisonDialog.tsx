//! Tune Comparison Dialog
//!
//! Shown when the tune on ECU differs from the tune in the project.
//! Allows user to choose which tune to use.

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, HardDrive, FileText, Loader } from "lucide-react";
import { Dialog, Button } from "../common";
import "./TuneComparisonDialog.css";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onUseProjectTune: () => void;
  onUseEcuTune: () => void;
}

export default function TuneComparisonDialog({
  isOpen,
  onClose,
  onUseProjectTune,
  onUseEcuTune,
}: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleUseProjectTune = async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke("write_project_tune_to_ecu");
      onUseProjectTune();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleUseEcuTune = async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke("save_tune_to_project");
      onUseEcuTune();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const titleNode = (
    <>
      <AlertTriangle size={20} className="warning-icon" />
      Tune Mismatch Detected
    </>
  );

  return (
    <Dialog
      open={isOpen}
      onClose={onClose}
      title={titleNode}
      size="md"
      className="tune-comparison-dialog"
      closeOnBackdrop={!loading}
      closeOnEscape={!loading}
    >
      <Dialog.Body className="tune-comparison-content">
        <p>
          The tune on the ECU differs from the tune in your project.
          Choose which tune to use:
        </p>

        {error && <div className="dialog-error">{error}</div>}

        <div className="tune-choice-buttons">
          <button
            className="tune-choice-button tune-choice-project"
            onClick={handleUseProjectTune}
            disabled={loading}
          >
            <div className="tune-choice-icon">
              <FileText size={32} />
            </div>
            <div className="tune-choice-content">
              <h3>Use Project Tune</h3>
              <p>Load the tune from your project file and write it to the ECU</p>
            </div>
            {loading && <Loader className="tune-choice-loader" size={20} />}
          </button>

          <button
            className="tune-choice-button tune-choice-ecu"
            onClick={handleUseEcuTune}
            disabled={loading}
          >
            <div className="tune-choice-icon">
              <HardDrive size={32} />
            </div>
            <div className="tune-choice-content">
              <h3>Use ECU Tune</h3>
              <p>Keep the tune currently on the ECU and update your project file</p>
            </div>
            {loading && <Loader className="tune-choice-loader" size={20} />}
          </button>
        </div>
      </Dialog.Body>

      <Dialog.Footer>
        <Button variant="secondary" onClick={onClose} disabled={loading}>Cancel</Button>
      </Dialog.Footer>
    </Dialog>
  );
}
