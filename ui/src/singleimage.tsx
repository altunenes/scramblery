import { useState, ChangeEvent, useEffect } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import BackButton from './comp/BackButton';
import { BlockControls, type BlockOptions } from './BlockControls';

import { FourierControls, type FourierOptions, type FrequencyRange } from './FourierControls';
interface SelectedImage {
  data: string;
  name: string;
  originalSize: {
    width: number;
    height: number;
  };
}

function SingleImage() {
  const [selectedImage, setSelectedImage] = useState<SelectedImage | null>(null);
  const [scrambledImage, setScrambledImage] = useState<string | null>(null);
  const [scrambleType, setScrambleType] = useState<'Pixel' | 'Fourier' | 'Block'>('Pixel');
  const [intensity, setIntensity] = useState(50);
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [useFaceDetection, setUseFaceDetection] = useState(false);
  const [backgroundMode, setBackgroundMode] = useState<'Include' | 'Exclude'>('Include');
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
  const handleImageUpload = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    try {
      const buffer = await file.arrayBuffer();
      const uint8Array = new Uint8Array(buffer);
      let binary = '';
      uint8Array.forEach(byte => {
        binary += String.fromCharCode(byte);
      });
      const base64String = btoa(binary);

      const img = new Image();
      img.src = URL.createObjectURL(file);
      await new Promise((resolve) => {
        img.onload = () => {
          setSelectedImage({
            data: base64String,
            name: file.name,
            originalSize: {
              width: img.width,
              height: img.height,
            },
          });
          URL.revokeObjectURL(img.src);
          resolve(null);
        };
      });

      setError(null);
    } catch (err) {
      console.error('Error loading image:', err);
      setError("Failed to load image");
    }
  };
  useEffect(() => {
    setFourierOptions(prev => ({
      ...prev,
      intensity: intensity / 100
    }));
  }, [intensity]);
  const handleScramble = async () => {
    if (!selectedImage) return;
    setIsProcessing(true);
    setError(null);
    try {
      const sliderIntensity = intensity / 100;
      let scrambleTypeOption;
      
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
      }

      const result = await invoke<string>('scramble_image', {
        imageData: selectedImage.data,
        options: {
          scramble_type: scrambleTypeOption,
          intensity: sliderIntensity,
          seed: null,
          face_detection: useFaceDetection ? {
            confidence_threshold: 0.7,
            expansion_factor: 1.0,
            background_mode: backgroundMode,
          } : null,
        }
      });
      setScrambledImage(result);
    } catch (err) {
      console.error('Scramble error:', err);
      setError(typeof err === 'string' ? err : 'Failed to scramble image');
    } finally {
      setIsProcessing(false);
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
  const handleSaveScrambled = async () => {
    if (!scrambledImage || !selectedImage) return;

    try {
      const filePath = await save({
        filters: [{
          name: 'Image',
          extensions: ['png'],
        }],
        defaultPath: selectedImage.name.replace(/\.[^/.]+$/, "") + '_scrambled.png',
      });

      if (filePath) {
        const binaryString = atob(scrambledImage);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }
        await writeFile(filePath, bytes);
      }
    } catch (err) {
      console.error('Error saving file:', err);
      setError('Failed to save image');
    }
  };

  return (
    <div className="app-container">
      <BackButton />
      
      {/* Main Content Area */}
      <div className="flex flex-col gap-4">
        {/* Top Control Panel */}
        <div className="controls-panel">
          <h2>Image Scrambler</h2>
          
          {/* File Upload Section */}
          <div className="file-upload-container">
            <label>Select Image</label>
            <input
              type="file"
              accept="image/*"
              onChange={handleImageUpload}
              className="file-input"
            />
          </div>

          {/* Core Controls */}
          <div className="flex gap-4 mt-4">
            {/* Left side - Basic Controls */}
            <div className="flex-1">
              <div className="scramble-type-control mb-4">
                <label>Scramble Method:</label>
                <select
                  value={scrambleType}
                  onChange={(e) => setScrambleType(e.target.value as 'Pixel' | 'Fourier' | 'Block')}
                  className="select-input"
                  >
                  <option value="Pixel">Pixel Scrambling</option>
                  <option value="Fourier">Fourier Scrambling</option>
                  <option value="Block">Block Scrambling</option>
                </select>
              </div>
              {/*Block Controls */}
              {scrambleType === 'Block' && (
                <BlockControls
                  options={blockOptions}
                  onChange={setBlockOptions}
                />
              )}
              {/* Always Visible Important Controls */}
              <div className="intensity-control mb-4">
                <label>Intensity: {intensity}%</label>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={intensity}
                  onChange={(e) => setIntensity(Number(e.target.value))}
                />
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
              </div>
            </div>

            {/* Right side - Fourier Controls when selected */}
            <div className="flex-1">
              {scrambleType === 'Fourier' && (
                <FourierControls
                  options={fourierOptions}
                  onChange={setFourierOptions}
                />
              )}
            </div>
          </div>
        </div>

        {/* Action Buttons*/}
        <div className="controls-panel py-3">
        <div className="action-buttons">
          <button
            onClick={handleScramble}
            disabled={!selectedImage || isProcessing}
            className="scramble-button"
          >
            {isProcessing ? "Processing..." : "Scramble Image"}
          </button>
          
          {scrambledImage && (
            <button onClick={handleSaveScrambled} className="save-button">
              Save Result
            </button>
          )}
        </div>
      </div>
        {error && <div className="error-message">{error}</div>}

      {/* Image Preview Section */}
      {selectedImage && (
        <div className="image-comparison-container">
          {/* Original Image Panel */}
          <div className="image-panel original-panel">
            <div className="panel-header">
              <h3>Original Image</h3>
              <p>{selectedImage.originalSize.width}x{selectedImage.originalSize.height}</p>
            </div>
            <div className="image-container">
              <img
                src={`data:image/png;base64,${selectedImage.data}`}
                alt="Original"
              />
            </div>
          </div>

          {/* Scrambled Image Panel */}
          <div className="image-panel scrambled-panel">
            <div className="panel-header">
              <h3>Scrambled Result</h3>
              <p>{selectedImage.originalSize.width}x{selectedImage.originalSize.height}</p>
            </div>
            <div className="image-container">
              {scrambledImage ? (
                <img
                  src={`data:image/png;base64,${scrambledImage}`}
                  alt="Scrambled"
                />
              ) : (
                <div className="placeholder">
                  <span>Scrambled preview will appear here</span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  </div>
  );
}

export default SingleImage;