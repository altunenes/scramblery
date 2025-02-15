import { useState } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { open} from '@tauri-apps/plugin-dialog';

interface ProcessingResult {
  input_path: string;
  output_path: string;
  success: boolean;
  error?: string;
}

interface BatchProgress {
  total_files: number;
  processed_files: number;
  current_file?: string;
}

function FolderProcess() {
  const [inputDir, setInputDir] = useState<string | null>(null);
  const [outputDir, setOutputDir] = useState<string | null>(null);
  const [intensity, setIntensity] = useState(50);
  const [useFaceDetection, setUseFaceDetection] = useState(false);
  const [backgroundMode, setBackgroundMode] = useState<'Include' | 'Exclude'>('Include');
  const [isProcessing, setIsProcessing] = useState(false);
  const [progress, setProgress] = useState<BatchProgress | null>(null);
  const [results, setResults] = useState<ProcessingResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  const handleSelectInputDir = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Input Directory',
      });
      if (selected) {
        setInputDir(selected as string);
      }
    } catch (err) {
      console.error('Error selecting input directory:', err);
      setError('Failed to select input directory');
    }
  };

  const handleSelectOutputDir = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Output Directory',
      });
      if (selected) {
        setOutputDir(selected as string);
      }
    } catch (err) {
      console.error('Error selecting output directory:', err);
      setError('Failed to select output directory');
    }
  };

  const handleProcess = async () => {
    if (!inputDir || !outputDir) return;
    
    setIsProcessing(true);
    setError(null);
    setResults([]);
    setProgress(null);

    try {
      const options = {
        input_dir: inputDir,
        output_dir: outputDir,
        scramble_options: {
          intensity: intensity / 100,
          seed: null,
          face_detection: useFaceDetection ? {
            confidence_threshold: 0.7,
            expansion_factor: 1.0,
            background_mode: backgroundMode,
          } : null,
        },
      };

      const results = await invoke<ProcessingResult[]>('process_directory', {
        options,
      });

      setResults(results);
    } catch (err) {
      console.error('Processing error:', err);
      setError(typeof err === 'string' ? err : 'Failed to process images');
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <div className="app-container">
      <div className="controls-panel">
        <h2>Batch Image Processing</h2>
        
        <div className="directory-selection">
          <div className="input-group">
            <label>Input Directory</label>
            <div className="directory-input">
              <input
                type="text"
                value={inputDir || ''}
                readOnly
                className="directory-path"
              />
              <button onClick={handleSelectInputDir} className="select-dir-button">
                Select
              </button>
            </div>
          </div>

          <div className="input-group">
            <label>Output Directory</label>
            <div className="directory-input">
              <input
                type="text"
                value={outputDir || ''}
                readOnly
                className="directory-path"
              />
              <button onClick={handleSelectOutputDir} className="select-dir-button">
                Select
              </button>
            </div>
          </div>
        </div>

        {/* Reuse the same controls from SingleImage component */}
        <div className="face-detection-control">
          <div className="checkbox-control">
            <input
              type="checkbox"
              id="face-detection"
              checked={useFaceDetection}
              onChange={(e) => setUseFaceDetection(e.target.checked)}
            />
            <label htmlFor="face-detection">Use Face Detection</label>
          </div>
          
          {useFaceDetection && (
            <div className="background-mode-control">
              <label>Background Mode:</label>
              <select
                value={backgroundMode}
                onChange={(e) => setBackgroundMode(e.target.value as 'Include' | 'Exclude')}
                className="select-input"
              >
                <option value="Include">Keep Background (Scramble Faces Only)</option>
                <option value="Exclude">Exclude Background (Faces Only)</option>
              </select>
            </div>
          )}
        </div>

        <div className="intensity-control">
          <label>Intensity: {intensity}%</label>
          <input
            type="range"
            min="0"
            max="100"
            value={intensity}
            onChange={(e) => setIntensity(Number(e.target.value))}
          />
        </div>

        <button
          onClick={handleProcess}
          disabled={!inputDir || !outputDir || isProcessing}
          className="process-button"
        >
          {isProcessing ? "Processing..." : "Process Images"}
        </button>

        {error && <div className="error-message">{error}</div>}
      </div>

      {/* Progress and Results */}
      {(isProcessing || results.length > 0) && (
        <div className="results-panel">
          {progress && (
            <div className="progress-bar">
              <div 
                className="progress-fill"
                style={{ width: `${(progress.processed_files / progress.total_files) * 100}%` }}
              />
              <span className="progress-text">
                {progress.processed_files} / {progress.total_files} images processed
              </span>
            </div>
          )}

          {results.length > 0 && (
            <div className="results-list">
              <h3>Processing Results</h3>
              <div className="results-summary">
                <p>Successfully processed: {results.filter(r => r.success).length}</p>
                <p>Failed: {results.filter(r => !r.success).length}</p>
              </div>
              
              {results.filter(r => !r.success).map((result, index) => (
                <div key={index} className="result-item error">
                  <span className="file-name">{result.input_path}</span>
                  <span className="error-message">{result.error}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default FolderProcess;