import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Plus, Trash2, Save, Calculator, AlertCircle, CheckCircle } from 'lucide-react';
import { Dialog, Button, FormField } from '../common';
import './MathChannelsDialog.css';

interface UserMathChannel {
  name: string;
  units: string;
  expression: string;
}

interface MathChannelsDialogProps {
  onClose: () => void;
}

export default function MathChannelsDialog({ onClose }: MathChannelsDialogProps) {
  const [channels, setChannels] = useState<UserMathChannel[]>([]);
  const [editingId, setEditingId] = useState<string | null>(null); // Use name as ID
  
  // Form state
  const [name, setName] = useState('');
  const [units, setUnits] = useState('');
  const [expression, setExpression] = useState('');
  
  const [validationMsg, setValidationMsg] = useState<string | null>(null);
  const [isValid, setIsValid] = useState(false);
  const [isDirty, setIsDirty] = useState(false);

  useEffect(() => {
    loadChannels();
  }, []);

  // Debounced validation
  useEffect(() => {
    if (!expression.trim()) {
        setIsValid(false);
        setValidationMsg(null);
        return;
    }
    // Don't validate if we just loaded the form and haven't touched it (unless it's new)
    if (editingId && !isDirty && editingId !== '__NEW__') {
        const current = channels.find(c => c.name === editingId);
        if (current && current.expression === expression) {
            setIsValid(true);
            return;
        }
    }

    const timer = setTimeout(() => validate(expression), 500);
    return () => clearTimeout(timer);
  }, [expression, editingId]);

  const loadChannels = async () => {
    try {
      const result = await invoke<UserMathChannel[]>('get_math_channels');
      setChannels(result);
    } catch (err) {
      console.error('Failed to load channels:', err);
    }
  };

  const resetForm = () => {
    setName('');
    setUnits('');
    setExpression('');
    setEditingId(null);
    setValidationMsg(null);
    setIsValid(false);
    setIsDirty(false);
  };

  const handleSelect = (channel: UserMathChannel) => {
    setEditingId(channel.name);
    setName(channel.name);
    setUnits(channel.units);
    setExpression(channel.expression);
    setValidationMsg(null);
    setIsValid(true); // Assume saved channels are valid
    setIsDirty(false);
  };

  const handleNew = () => {
    resetForm();
    setName('NewChannel');
    setEditingId('__NEW__'); 
    setIsDirty(true);
  };

  const handleDelete = async (targetName: string) => {
    if (!confirm(`Delete channel "${targetName}"?`)) return;
    
    try {
      await invoke('delete_math_channel', { name: targetName });
      await loadChannels();
      if (editingId === targetName) {
        resetForm();
      }
    } catch (err) {
      alert(`Failed to delete: ${err}`);
    }
  };

  const validate = async (expr: string) => {
    try {
      await invoke('validate_math_expression', { expr });
      setValidationMsg("Valid expression");
      setIsValid(true);
    } catch (err) {
      setValidationMsg(String(err));
      setIsValid(false);
    }
  };

  const handleSave = async () => {
    if (!name.trim() || !isValid) return;

    const channel: UserMathChannel = {
      name: name.trim(),
      units: units.trim(),
      expression: expression.trim()
    };

    try {
      await invoke('set_math_channel', { channel });
      await loadChannels();
      setEditingId(channel.name); // Switch to edit mode for the saved channel
      setIsDirty(false);
    } catch (err) {
      alert(`Failed to saving: ${err}`);
    }
  };

  const titleNode = (
    <>
      <Calculator size={18} /> Math Channels
    </>
  );

  return (
    <Dialog
      open
      onClose={onClose}
      title={titleNode}
      size="xl"
      className="math-channels-dialog"
    >
      <Dialog.Body className="math-channels-body">
          <div className="channels-list">
            <div className="channels-list-header">
              <h3>Defined Channels</h3>
              <button className="btn-add" onClick={handleNew} title="Add New Channel">
                <Plus size={18} />
              </button>
            </div>
            <div className="channels-list-items">
              {channels.map(c => (
                <div 
                  key={c.name} 
                  className={`channel-item ${editingId === c.name ? 'active' : ''}`}
                  onClick={() => handleSelect(c)}
                >
                  <span className="channel-name">{c.name}</span>
                  <span className="channel-expr">{c.expression}</span>
                </div>
              ))}
              {channels.length === 0 && (
                <div className="empty-channels">
                  No custom channels defined
                </div>
              )}
            </div>
          </div>

          <div className="channel-editor">
            {!editingId ? (
              <div className="editor-placeholder">
                <Calculator size={48} />
                <p>Select a channel to edit or create a new one</p>
                <Button variant="primary" onClick={handleNew}>Create New Channel</Button>
              </div>
            ) : (
              <div className="editor-form">
                <div className="form-row">
                  <div style={{ flex: 2 }}>
                    <FormField label="Channel Name">
                      {(id) => (
                        <input
                          id={id}
                          value={name}
                          onChange={e => { setName(e.target.value); setIsDirty(true); }}
                          placeholder="e.g. Boost_PSI"
                          disabled={editingId !== '__NEW__' && editingId !== name}
                        />
                      )}
                    </FormField>
                  </div>
                  <div style={{ flex: 1 }}>
                    <FormField label="Units">
                      {(id) => (
                        <input
                          id={id}
                          value={units}
                          onChange={e => { setUnits(e.target.value); setIsDirty(true); }}
                          placeholder="psi"
                        />
                      )}
                    </FormField>
                  </div>
                </div>

                <FormField label="Expression">
                  {(id) => (
                    <input
                      id={id}
                      className="font-mono"
                      value={expression}
                      onChange={e => { setExpression(e.target.value); setIsDirty(true); }}
                      placeholder="(map - 100) * 0.145"
                    />
                  )}
                </FormField>

                {expression && (
                  <div className={`validation-status ${isValid ? 'valid' : 'invalid'}`}>
                    {isValid ? <CheckCircle size={16} /> : <AlertCircle size={16} />}
                    {validationMsg || "Checking..."}
                  </div>
                )}

                <div className="editor-actions">
                  <Button 
                    variant="primary"
                    onClick={handleSave} 
                    disabled={!isValid || !name || !isDirty}
                    leadingIcon={<Save size={14} />}
                  >
                    Save Channel
                  </Button>
                  {editingId !== '__NEW__' && (
                    <Button 
                      variant="danger"
                      onClick={() => handleDelete(editingId)}
                      leadingIcon={<Trash2 size={14} />}
                    >
                      Delete
                    </Button>
                  )}
                </div>
                
                <div className="formula-help">
                    <h4>Formula Reference</h4>
                    <ul>
                        <li>Use channel names: <code>rpm</code>, <code>map</code>, <code>tps</code>, <code>clt</code></li>
                        <li>Basic operators: <code>+</code>, <code>-</code>, <code>*</code>, <code>/</code>, <code>%</code></li>
                        <li>Comparison: <code>&gt;</code>, <code>&lt;</code>, <code>==</code> (returns 1.0 or 0.0)</li>
                        <li>Functions: <code>sin(x)</code>, <code>cos(x)</code>, <code>min(a,b)</code>, <code>max(a,b)</code></li>
                        <li>Logic: <code>(conditions) ? 1 : 0</code> for simple logic</li>
                    </ul>
                </div>
              </div>
            )}
          </div>
      </Dialog.Body>

      <Dialog.Footer>
        <Button variant="secondary" onClick={onClose}>Close</Button>
      </Dialog.Footer>
    </Dialog>
  );
}
