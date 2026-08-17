/**
 * HelpViewer - Displays context-sensitive help from INI files.
 */

import { ExternalLink, Book } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Dialog, Button } from '../common';
import './HelpViewer.css';

/** Help topic data from INI files */
export interface HelpTopicData {
  name: string;
  title: string;
  web_url?: string;
  text_lines: string[];
}

interface HelpViewerProps {
  topic: HelpTopicData;
  onClose: () => void;
  onOpenManual?: () => void;
}

export default function HelpViewer({ topic, onClose, onOpenManual }: HelpViewerProps) {
  const handleWebHelp = async () => {
    if (topic.web_url) {
      try {
        await openUrl(topic.web_url);
      } catch (err) {
        console.error('Failed to open URL:', err);
      }
    }
  };

  const htmlContent = topic.text_lines.join('\n');

  return (
    <Dialog open onClose={onClose} title={topic.title} size="md" className="help-viewer-dialog">
      <Dialog.Body className="help-viewer-content">
        {topic.text_lines.length > 0 ? (
          <div
            className="help-text"
            dangerouslySetInnerHTML={{ __html: htmlContent }}
          />
        ) : (
          <p className="help-no-content">No help content available.</p>
        )}
      </Dialog.Body>
      <Dialog.Footer>
        {onOpenManual && (
          <Button variant="secondary" onClick={onOpenManual} leadingIcon={<Book size={16} />}>
            User Manual
          </Button>
        )}
        {topic.web_url && (
          <Button variant="secondary" onClick={handleWebHelp} leadingIcon={<ExternalLink size={16} />}>
            Open Web Help
          </Button>
        )}
        <Button variant="primary" onClick={onClose}>Close</Button>
      </Dialog.Footer>
    </Dialog>
  );
}
