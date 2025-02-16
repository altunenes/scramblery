import { useState} from 'react';
import { invoke } from "@tauri-apps/api/core";
import { open} from '@tauri-apps/plugin-dialog';

interface VideoProcessingOptions {
  input_path: string;
  output_path: string;
  scramble_options: {
    intensity: number;
    seed: number | null;
    face_detection: {
      confidence_threshold: number;
      expansion_factor: number;
      background_mode: 'Include' | 'Exclude';
    } | null;
  };
}

function VideoProcess() {
  const [inputPath, setInputPath] = useState<string | null>(null);
  const [intensity, setIntensity] = useState(50);
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [useFaceDetection, setUseFaceDetection] = useState(false);
  const [backgroundMode, setBackgroundMode] = useState<'Include' | 'Exclude'>('Include');
  const [progress, setProgress] = useState<number | null>(null);

  const handleSelectVideo = async () => {
    try {
      const selected = await open({
        filters: [{
          name: 'Video',
          extensions: ['mp4', 'avi', 'mov', 'mkv']
        }],
        multiple: false,
      });
      
      if (selected) {
        setInputPath(selected as string);
      }
    } catch (err) {
      console.error('Error selecting video:', err);
      setError('Failed to select video');
    }
  };

  const handleProcess = async () => {
    if (!inputPath) return;
    
    setIsProcessing(true);
    setError(null);
    setProgress(0);

    try {
      const outputPath = inputPath.replace(/\.[^/.]+$/, "") + '_scrambled.mp4';
      
      const options: VideoProcessingOptions = {
        input_path: inputPath,
        output_path: outputPath,
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

      await invoke('process_video', { options });
      
    } catch (err) {
      console.error('Processing error:', err);
      setError(typeof err === 'string' ? err : 'Failed to process video');
    } finally {
      setIsProcessing(false);
      setProgress(null);
    }
  };

  return (
    <div className="app-container">
      <div className="controls-panel">
        <h2>Video Scrambler</h2>
        
        <div className="file-selection">
          <div className="input-group">
            <label>Input Video</label>
            <div className="file-input">
              <input
                type="text"
                value={inputPath || ''}
                readOnly
                className="file-path"
              />
              <button onClick={handleSelectVideo} className="select-file-button">
                Select Video
              </button>
            </div>
          </div>
        </div>

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
          disabled={!inputPath || isProcessing}
          className="process-button"
        >
          {isProcessing ? "Processing..." : "Process Video"}
        </button>

        {error && <div className="error-message">{error}</div>}
        
        {progress !== null && (
          <div className="progress-bar">
            <div 
              className="progress-fill"
              style={{ width: `${progress}%` }}
            />
            <span className="progress-text">
              {Math.round(progress)}% complete
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

export default VideoProcess;