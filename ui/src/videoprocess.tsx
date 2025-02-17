import { useState, useEffect } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { open} from '@tauri-apps/plugin-dialog';
import { FourierControls, type FourierOptions, type FrequencyRange } from './FourierControls';

interface VideoProcessingOptions {
  input_path: string;
  output_path: string;
  scramble_options: {
    scramble_type: 'Pixel' | {
      Fourier: {
        frequency_range: any;
        phase_scramble: boolean;
        magnitude_scramble: boolean;
        padding_mode: string;
        intensity: number;
        grayscale: boolean;
      }
    };
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
  const [scrambleType, setScrambleType] = useState<'Pixel' | 'Fourier'>('Pixel');
  const [fourierOptions, setFourierOptions] = useState<FourierOptions>({
    frequency_range: 'All',
    phase_scramble: true,
    magnitude_scramble: false,
    padding_mode: 'Reflect',
    intensity: 0.5,
    grayscale: false
  });
  useEffect(() => {
    setFourierOptions(prev => ({
      ...prev,
      intensity: intensity / 100
    }));
  }, [intensity]);
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
  const formatFrequencyRange = (range: FrequencyRange) => {
    if (range === 'All') return 'All';
    if ('HighPass' in range) return { HighPass: range.HighPass.cutoff };
    if ('LowPass' in range) return { LowPass: range.LowPass.cutoff };
    if ('BandPass' in range) return { 
      BandPass: { low: range.BandPass.low, high: range.BandPass.high } 
    };
    return 'All';
  };
  const handleProcess = async () => {
    if (!inputPath) return;
    
    setIsProcessing(true);
    setError(null);
    setProgress(0);

    try {
      const outputPath = inputPath.replace(/\.[^/.]+$/, "") + '_scrambled.mp4';
      const sliderIntensity = intensity / 100;
      
      const options: VideoProcessingOptions = {
        input_path: inputPath,
        output_path: outputPath,
        scramble_options: {
          scramble_type: scrambleType === 'Pixel' 
            ? 'Pixel'
            : {
                Fourier: {
                  frequency_range: formatFrequencyRange(fourierOptions.frequency_range),
                  phase_scramble: fourierOptions.phase_scramble,
                  magnitude_scramble: fourierOptions.magnitude_scramble,
                  padding_mode: fourierOptions.padding_mode,
                  intensity: sliderIntensity,
                  grayscale: fourierOptions.grayscale
                }
              },
          intensity: sliderIntensity,
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
        
        <div className="scramble-type-control">
          <label>Scramble Method:</label>
          <select
            value={scrambleType}
            onChange={(e) => setScrambleType(e.target.value as 'Pixel' | 'Fourier')}
            className="select-input"
          >
            <option value="Pixel">Pixel Scrambling</option>
            <option value="Fourier">Fourier Scrambling</option>
          </select>
        </div>

        {scrambleType === 'Fourier' && (
          <FourierControls
            options={fourierOptions}
            onChange={setFourierOptions}
          />
        )}
        
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