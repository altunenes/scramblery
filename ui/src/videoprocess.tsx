import { useState, useEffect } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { open} from '@tauri-apps/plugin-dialog';
import { FourierControls, type FourierOptions, type FrequencyRange } from './FourierControls';
import { BlockControls, type BlockOptions } from './BlockControls';
import BackButton from './comp/BackButton';
import { listen } from '@tauri-apps/api/event';
import { BlurControls, type BlurOptions } from './BlurControls';
import { DiffeomorphicControls, type DiffeomorphicOptions } from './DiffeomorphicControls';

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
    }
  | {
      Diffeomorphic: {
        max_distortion: number;
        n_steps: number;
        n_comp: number;
      }
    };

interface VideoProcessingOptions {
  input_path: string;
  output_path: string;
  scramble_options: {
    scramble_type: ScrambleTypeOption;
    intensity: number;
    seed: number | null;
    face_detection: {
      confidence_threshold: number;
      expansion_factor: number;
      background_mode: 'Include' | 'Exclude';
    } | null;
  };
  temporal_coherence: {
    export_flow: boolean;
    flow_output_dir: string | null;
    keyframe_interval: number;
    blend_frames: number;
  } | null;
}

function VideoProcess() {
  const [inputPath, setInputPath] = useState<string | null>(null);
  const [intensity, setIntensity] = useState(50);
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [useFaceDetection, setUseFaceDetection] = useState(false);
  const [backgroundMode, setBackgroundMode] = useState<'Include' | 'Exclude'>('Include');
  const [progress, setProgress] = useState<number | null>(null);
  const [scrambleType, setScrambleType] = useState<'Pixel' | 'Fourier' | 'Block' | 'Blur' | 'Diffeomorphic'>('Pixel');
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
    interpolate_edges: false,
    padding_mode: 'Reflect'
  });
  const [blurOptions, setBlurOptions] = useState<BlurOptions>({
    sigma: 5.0,
  });
  const [diffeomorphicOptions, setDiffeomorphicOptions] = useState<DiffeomorphicOptions>({
    max_distortion: 5.0,
    n_steps: 20,
    n_comp: 5,
  });
  const [useTemporalCoherence, setUseTemporalCoherence] = useState(false);
  const [exportFlow, setExportFlow] = useState(false);
  const [flowOutputDir, setFlowOutputDir] = useState<string | null>(null);
  const [keyframeInterval, setKeyframeInterval] = useState(30);
  const [blendFrames, setBlendFrames] = useState(0);

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
        case 'Diffeomorphic':
          scrambleTypeOption = {
            Diffeomorphic: diffeomorphicOptions
          };
          break;
      }

      const options: VideoProcessingOptions = {
        input_path: inputPath,
        output_path: outputPath,
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
        temporal_coherence: useTemporalCoherence ? {
          export_flow: exportFlow,
          flow_output_dir: exportFlow ? flowOutputDir : null,
          keyframe_interval: keyframeInterval,
          blend_frames: blendFrames,
        } : null,
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

  useEffect(() => {
    const unlisten = listen<number>('video-progress', (event) => {
      setProgress(event.payload);
    });

    return () => {
      unlisten.then(unsubscribe => unsubscribe());
    };
  }, []);

  return (
    <div className="app-container">
      <BackButton />
      <div className="controls-panel">
        <h2>Video Scrambler</h2>

        {/* Scramble Method */}
        <div className="scramble-type-control">
          <label>Scramble Method</label>
          <select
            value={scrambleType}
            onChange={(e) => setScrambleType(e.target.value as 'Pixel' | 'Fourier' | 'Block' | 'Blur' | 'Diffeomorphic')}
            className="select-input"
          >
            <option value="Pixel">Pixel Scrambling</option>
            <option value="Fourier">Fourier Phase Scrambling</option>
            <option value="Block">Block Scrambling</option>
            <option value="Blur">Gaussian Blur</option>
            <option value="Diffeomorphic">Diffeomorphic Warp</option>
          </select>
        </div>

        {/* Method-specific controls */}
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

        {scrambleType === 'Diffeomorphic' && (
          <DiffeomorphicControls
            options={diffeomorphicOptions}
            onChange={setDiffeomorphicOptions}
          />
        )}

        {/* Intensity — only for Pixel and Fourier */}
        {(scrambleType === 'Pixel' || scrambleType === 'Fourier') && (
          <div className="intensity-control">
            <label>Intensity</label>
            <div className="slider-row">
              <input
                type="range"
                min="0"
                max="100"
                value={intensity}
                onChange={(e) => setIntensity(Number(e.target.value))}
              />
              <span className="slider-value">{intensity}%</span>
            </div>
          </div>
        )}

        {/* File Selection */}
        <div className="file-selection">
          <div className="input-group">
            <label>Input Video</label>
            <div className="file-input">
              <input
                type="text"
                value={inputPath || ''}
                readOnly
                className="file-path"
                placeholder="No file selected"
              />
              <button onClick={handleSelectVideo} className="select-file-button">
                Browse
              </button>
            </div>
          </div>
        </div>

        {/* Face Detection */}
        <div className="face-detection-control">
          <div className="checkbox-control">
            <input
              type="checkbox"
              id="face-detection"
              checked={useFaceDetection}
              onChange={(e) => setUseFaceDetection(e.target.checked)}
            />
            <label htmlFor="face-detection">Face Detection</label>
          </div>

          {useFaceDetection && (
            <div className="background-mode-control">
              <label>Background Mode</label>
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

        {/* Temporal Coherence */}
        <div className="temporal-coherence-control">
          <div className="checkbox-control">
            <input
              type="checkbox"
              id="temporal-coherence"
              checked={useTemporalCoherence}
              onChange={(e) => setUseTemporalCoherence(e.target.checked)}
            />
            <label htmlFor="temporal-coherence">Temporal Coherence (Optical Flow)</label>
          </div>

          {useTemporalCoherence && (
            <div className="temporal-coherence-options">
              <p className="info-note">
                Uses SEA-RAFT to preserve original motion in scrambled output. Keyframes are scrambled fresh; frames in between are warped to follow original motion.
              </p>

              <div className="control-group">
                <label>Keyframe Interval</label>
                <div className="slider-row">
                  <input
                    type="range"
                    min="0"
                    max="120"
                    value={keyframeInterval}
                    onChange={(e) => setKeyframeInterval(Number(e.target.value))}
                  />
                  <span className="slider-value">{keyframeInterval}</span>
                </div>
                <p className="info-note">
                  Fresh scramble every N frames. 0 = only first frame. Higher = smoother motion but more warp accumulation.
                </p>
              </div>

              <div className="control-group">
                <label>Keyframe Blend</label>
                <div className="slider-row">
                  <input
                    type="range"
                    min="0"
                    max="10"
                    value={blendFrames}
                    onChange={(e) => setBlendFrames(Number(e.target.value))}
                  />
                  <span className="slider-value">{blendFrames}</span>
                </div>
                <p className="info-note">
                  Crossfade frames at keyframe boundaries. 0 = abrupt switch.
                </p>
              </div>

              <div className="checkbox-control">
                <input
                  type="checkbox"
                  id="export-flow"
                  checked={exportFlow}
                  onChange={(e) => setExportFlow(e.target.checked)}
                />
                <label htmlFor="export-flow">Export Flow Files (.flo)</label>
              </div>

              {exportFlow && (
                <div className="flow-output-dir">
                  <label>Flow Output Directory</label>
                  <div className="file-input">
                    <input
                      type="text"
                      value={flowOutputDir || ''}
                      readOnly
                      className="file-path"
                      placeholder="Select directory"
                    />
                    <button
                      onClick={async () => {
                        try {
                          const selected = await open({ directory: true });
                          if (selected) setFlowOutputDir(selected as string);
                        } catch (err) {
                          console.error('Error selecting directory:', err);
                        }
                      }}
                      className="select-file-button"
                    >
                      Browse
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Process Button */}
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
              {Math.round(progress)}%
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

export default VideoProcess;
