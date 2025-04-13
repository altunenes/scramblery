import { useState, useEffect } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { FourierControls, type FourierOptions, type FrequencyRange } from './FourierControls';
import { BlockControls, type BlockOptions } from './BlockControls';
import BackButton from './comp/BackButton';
import { BlurControls, type BlurOptions } from './BlurControls';

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

type ScrambleTypeOption = 
  | 'Pixel'
  | { 
      Fourier: {
        frequency_range: any;
        phase_scramble: boolean;
        magnitude_scramble: boolean;
        padding_mode: string;
        intensity: number;
        grayscale: boolean;
      }
    }
  | {
      Block: {
        block_size: [number, number];
        interpolate_edges: boolean;
        padding_mode: 'Zero' | 'Reflect' | 'Wrap';
      }
    }
  | {
      Blur: {
        sigma: number;
      }
    };

    const ProgressPanel = ({ progress, results }: { progress: BatchProgress | null, results: ProcessingResult[] }) => {
      if (!progress && results.length === 0) return null;
    
      const successCount = results.filter(r => r.success).length;
      const failCount = results.filter(r => !r.success).length;
      const percentage = progress ? (progress.processed_files / progress.total_files) * 100 : 0;
    
      return (
        <div className="results-panel">
          <div className="progress-section">
            {progress && (
              <>
                <div className="progress-header">
                  <h3>Processing Progress</h3>
                  {progress?.current_file && (
                    <p className="current-file">
                      Processing: {progress.processed_files} / {progress.total_files} images
                    </p>
                  )}
                </div>
                
                <div className="progress-bar">
                  <div 
                    className="progress-fill"
                    style={{ width: `${percentage}%` }}
                  />
                  <span className="progress-text">
                    {Math.round(percentage)}% complete
                  </span>
                </div>
              </>
            )}
    
            {results.length > 0 && (
              <div className="progress-stats">
                <div className="stat-item">
                  <span className="stat-label">Completed</span>
                  <span className="stat-value">{results.length}</span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">Success</span>
                  <span className="stat-value success">{successCount}</span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">Failed</span>
                  <span className="stat-value error">{failCount}</span>
                </div>
              </div>
            )}
          </div>
        </div>
      );
    };

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
  const [scrambleType, setScrambleType] = useState<'Pixel' | 'Fourier' | 'Block' | 'Blur'>('Pixel');
  const [fourierOptions, setFourierOptions] = useState<FourierOptions>({
    frequency_range: 'All',
    phase_scramble: true,
    magnitude_scramble: false,
    padding_mode: 'Reflect',
    intensity: 0.5,
    grayscale: false
  });
  const [blockOptions, setBlockOptions] = useState<BlockOptions>({
    block_size: [32, 32],
    interpolate_edges: true,
    padding_mode: 'Reflect'
  });
  const [blurOptions, setBlurOptions] = useState<BlurOptions>({
    sigma: 5.0,
  });
  
  useEffect(() => {
    const unlisten = listen<BatchProgress>('batch-progress', (event) => {
      setProgress(event.payload);
    });

    return () => {
      unlisten.then(unsubscribe => unsubscribe());
    };
  }, []);

  useEffect(() => {
    setFourierOptions(prev => ({
      ...prev,
      intensity: intensity / 100
    }));
  }, [intensity]);

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
    if (!inputDir || !outputDir) return;
    
    setIsProcessing(true);
    setError(null);
    setResults([]);
    setProgress(null);
  
    try {
      const sliderIntensity = intensity / 100;
      
      let scrambleTypeOption: ScrambleTypeOption;
      switch (scrambleType) {
        case 'Pixel':
          scrambleTypeOption = 'Pixel';
          break;
        case 'Fourier':
          scrambleTypeOption = {
            Fourier: {
              frequency_range: formatFrequencyRange(fourierOptions.frequency_range),
              phase_scramble: fourierOptions.phase_scramble,
              magnitude_scramble: fourierOptions.magnitude_scramble,
              padding_mode: fourierOptions.padding_mode,
              intensity: sliderIntensity,
              grayscale: fourierOptions.grayscale
            }
          };
          break;
        case 'Block':
          scrambleTypeOption = {
            Block: blockOptions
          };
          break;
        case 'Blur':
          scrambleTypeOption = {
            Blur: blurOptions
          };
          break;
      }
  
      const options = {
        input_dir: inputDir,
        output_dir: outputDir,
        scramble_options: {
          scramble_type: scrambleTypeOption,
          intensity: sliderIntensity,
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
      <BackButton />
      <div className="controls-panel">
        <h2>Batch Image Processing</h2>
        <div className="scramble-type-control">
          <label>Scramble Method:</label>
          <select
            value={scrambleType}
            onChange={(e) => setScrambleType(e.target.value as 'Pixel' | 'Fourier' | 'Block' | 'Blur')}
            className="select-input"
          >
            <option value="Pixel">Pixel Scrambling</option>
            <option value="Fourier">Fourier Scrambling</option>
            <option value="Block">Block Scrambling</option>
            <option value="Blur">Gaussian Blur</option>
          </select>
        </div>

        {scrambleType === 'Fourier' && (
          <FourierControls
            options={fourierOptions}
            onChange={setFourierOptions}
          />
        )}

        {scrambleType === 'Block' && (
          <BlockControls
            options={blockOptions}
            onChange={setBlockOptions}
          />
        )}

        {scrambleType === 'Blur' && (
          <BlurControls
            options={blurOptions}
            onChange={setBlurOptions}
          />
        )}
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

      {/* New Progress and Results Panel */}
      {(isProcessing || results.length > 0) && (
        <ProgressPanel progress={progress} results={results} />
      )}
    </div>
  );
}

export default FolderProcess;