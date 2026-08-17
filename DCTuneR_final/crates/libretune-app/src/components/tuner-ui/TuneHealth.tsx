import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './TuneHealth.css';

/** Region health score from the backend */
interface RegionHealth {
  name: string;
  region_type: string;
  score: number;
  coverage_score: number;
  smoothness_score: number;
  monotonicity_score: number;
  cell_count: number;
  row_range: [number, number];
  col_range: [number, number];
}

/** Full health report from get_tune_health_report */
interface TuneHealthReport {
  overall_score: number;
  overall_grade: string;
  regions: RegionHealth[];
  recommendations: string[];
  total_cells: number;
  data_coverage_cells: number;
  data_coverage_percent: number;
}

/** Anomaly from get_tune_anomalies */
interface TuneAnomaly {
  row: number;
  col: number;
  value: number;
  expected_value: number;
  anomaly_type: string;
  severity: number;
  description: string;
}

/** Predicted cell from get_predicted_fills */
interface PredictedCell {
  row: number;
  col: number;
  predicted_value: number;
  current_value: number;
  confidence: number;
  method: string;
  neighbor_count: number;
}

interface TuneHealthProps {
  tableName: string;
}

/** Grade color mapping */
function gradeColor(grade: string): string {
  switch (grade) {
    case 'A': return '#4caf50';
    case 'B': return '#8bc34a';
    case 'C': return '#ffc107';
    case 'D': return '#ff9800';
    case 'F': return '#f44336';
    default: return '#999';
  }
}

/** Severity color */
function severityColor(severity: number): string {
  if (severity >= 0.7) return '#f44336';
  if (severity >= 0.4) return '#ff9800';
  return '#ffc107';
}

/** Score bar component */
function ScoreBar({ label, score }: { label: string; score: number }) {
  const color = score >= 80 ? '#4caf50' : score >= 60 ? '#ffc107' : score >= 40 ? '#ff9800' : '#f44336';
  return (
    <div className="score-bar">
      <span className="score-bar-label">{label}</span>
      <div className="score-bar-track">
        <div className="score-bar-fill" style={{ width: `${score}%`, backgroundColor: color }} />
      </div>
      <span className="score-bar-value">{score}</span>
    </div>
  );
}

export function TuneHealthCard({ tableName }: TuneHealthProps) {
  const [report, setReport] = useState<TuneHealthReport | null>(null);
  const [anomalies, setAnomalies] = useState<TuneAnomaly[]>([]);
  const [predictions, setPredictions] = useState<PredictedCell[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'health' | 'anomalies' | 'predictions'>('health');

  const refresh = useCallback(async () => {
    if (!tableName) return;
    setLoading(true);
    setError(null);
    try {
      const [healthReport, anomalyList, predictionList] = await Promise.all([
        invoke<TuneHealthReport>('get_tune_health_report', { tableName }),
        invoke<TuneAnomaly[]>('get_tune_anomalies', { tableName }),
        invoke<PredictedCell[]>('get_predicted_fills', { tableName }),
      ]);
      setReport(healthReport);
      setAnomalies(anomalyList);
      setPredictions(predictionList);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [tableName]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (loading) {
    return (
      <div className="tune-health-card">
        <div className="tune-health-loading">Analyzing tune...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="tune-health-card">
        <div className="tune-health-error">{error}</div>
      </div>
    );
  }

  if (!report) return null;

  return (
    <div className="tune-health-card">
      <div className="tune-health-header">
        <div className="tune-health-grade" style={{ borderColor: gradeColor(report.overall_grade) }}>
          <span className="grade-letter" style={{ color: gradeColor(report.overall_grade) }}>
            {report.overall_grade}
          </span>
          <span className="grade-score">{report.overall_score}/100</span>
        </div>
        <div className="tune-health-summary">
          <h3>Tune Health</h3>
          <p className="coverage-text">
            {report.data_coverage_cells}/{report.total_cells} cells covered ({report.data_coverage_percent.toFixed(0)}%)
          </p>
          <div className="health-badges">
            <span className="badge anomaly-badge" onClick={() => setActiveTab('anomalies')}>
              {anomalies.length} anomal{anomalies.length === 1 ? 'y' : 'ies'}
            </span>
            <span className="badge prediction-badge" onClick={() => setActiveTab('predictions')}>
              {predictions.length} prediction{predictions.length === 1 ? '' : 's'}
            </span>
          </div>
        </div>
        <button className="refresh-btn" onClick={refresh} title="Refresh analysis">⟳</button>
      </div>

      {/* Tab navigation */}
      <div className="health-tabs">
        <button
          className={`health-tab ${activeTab === 'health' ? 'active' : ''}`}
          onClick={() => setActiveTab('health')}
        >
          Regions
        </button>
        <button
          className={`health-tab ${activeTab === 'anomalies' ? 'active' : ''}`}
          onClick={() => setActiveTab('anomalies')}
        >
          Anomalies ({anomalies.length})
        </button>
        <button
          className={`health-tab ${activeTab === 'predictions' ? 'active' : ''}`}
          onClick={() => setActiveTab('predictions')}
        >
          Predictions ({predictions.length})
        </button>
      </div>

      {/* Tab content */}
      <div className="health-tab-content">
        {activeTab === 'health' && (
          <div className="regions-panel">
            {report.regions.map((region, i) => (
              <div key={i} className="region-card">
                <div className="region-header">
                  <span className="region-name">{region.name}</span>
                  <span className="region-score">{region.score}</span>
                </div>
                <ScoreBar label="Coverage" score={region.coverage_score} />
                <ScoreBar label="Smoothness" score={region.smoothness_score} />
                <ScoreBar label="Monotonicity" score={region.monotonicity_score} />
                <div className="region-info">
                  {region.cell_count} cells
                </div>
              </div>
            ))}
            {report.recommendations.length > 0 && (
              <div className="recommendations">
                <h4>Recommendations</h4>
                <ul>
                  {report.recommendations.map((rec, i) => (
                    <li key={i}>{rec}</li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}

        {activeTab === 'anomalies' && (
          <div className="anomalies-panel">
            {anomalies.length === 0 ? (
              <div className="empty-state">No anomalies detected — table looks clean!</div>
            ) : (
              <div className="anomaly-list">
                {anomalies.slice(0, 50).map((a, i) => (
                  <div key={i} className="anomaly-item" style={{ borderLeftColor: severityColor(a.severity) }}>
                    <div className="anomaly-header">
                      <span className="anomaly-type">{formatAnomalyType(a.anomaly_type)}</span>
                      <span className="anomaly-cell">
                        [{a.row}, {a.col}]
                      </span>
                      <span
                        className="anomaly-severity"
                        style={{ color: severityColor(a.severity) }}
                      >
                        {(a.severity * 100).toFixed(0)}%
                      </span>
                    </div>
                    <div className="anomaly-desc">{a.description}</div>
                  </div>
                ))}
                {anomalies.length > 50 && (
                  <div className="more-items">...and {anomalies.length - 50} more</div>
                )}
              </div>
            )}
          </div>
        )}

        {activeTab === 'predictions' && (
          <div className="predictions-panel">
            {predictions.length === 0 ? (
              <div className="empty-state">No predictions available — run AutoTune to gather data first</div>
            ) : (
              <div className="prediction-list">
                {predictions.slice(0, 50).map((p, i) => (
                  <div key={i} className="prediction-item">
                    <div className="prediction-header">
                      <span className="prediction-cell">
                        [{p.row}, {p.col}]
                      </span>
                      <span className="prediction-method">{formatMethod(p.method)}</span>
                      <span className="prediction-confidence">
                        {(p.confidence * 100).toFixed(0)}% conf
                      </span>
                    </div>
                    <div className="prediction-values">
                      <span className="current-val">{p.current_value.toFixed(1)}</span>
                      <span className="arrow">→</span>
                      <span className="predicted-val">{p.predicted_value.toFixed(1)}</span>
                      <span className="delta">
                        ({(p.predicted_value - p.current_value) >= 0 ? '+' : ''}
                        {(p.predicted_value - p.current_value).toFixed(1)})
                      </span>
                    </div>
                  </div>
                ))}
                {predictions.length > 50 && (
                  <div className="more-items">...and {predictions.length - 50} more</div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function formatAnomalyType(type: string): string {
  switch (type) {
    case 'StatisticalOutlier': return 'Outlier';
    case 'MonotonicityViolation': return 'Monotonicity';
    case 'GradientDiscontinuity': return 'Gradient';
    case 'PhysicallyUnreasonable': return 'Unreasonable';
    case 'FlatRegion': return 'Flat Region';
    default: return type;
  }
}

function formatMethod(method: string): string {
  switch (method) {
    case 'BilinearInterpolation': return 'Bilinear';
    case 'NeighborWeighted': return 'Neighbors';
    case 'LinearExtrapolation': return 'Extrapolation';
    case 'PhysicsModel': return 'Physics';
    default: return method;
  }
}
