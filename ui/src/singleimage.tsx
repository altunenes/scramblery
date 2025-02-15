import { useState, ChangeEvent } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';

interface SelectedImage {
  data: string;
  name: string;
  originalSize: {
    width: number;
    height: number;
  };
}
interface ScrambleOptions {
  intensity: number;
  seed: number | null;
  face_detection: {
    confidence_threshold: number;
    expansion_factor: number;
    background_mode: 'Include' | 'Exclude';
  } | null;
}
function SingleImage() {
  const [selectedImage, setSelectedImage] = useState<SelectedImage | null>(null);
  const [scrambledImage, setScrambledImage] = useState<string | null>(null);
  const [intensity, setIntensity] = useState(50);
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [useFaceDetection, setUseFaceDetection] = useState(false);
  const [backgroundMode, setBackgroundMode] = useState<'Include' | 'Exclude'>('Include');


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

  const handleScramble = async () => {
    if (!selectedImage) return;
    setIsProcessing(true);
    setError(null);

    try {
      const options: ScrambleOptions = {
        intensity: intensity / 100,
        seed: null,
        face_detection: useFaceDetection ? {
          confidence_threshold: 0.7,
          expansion_factor: 1.0,
          background_mode: backgroundMode,
        } : null,
      };
      const result = await invoke<string>('scramble_image', {
        imageData: selectedImage.data,
        options,
      });
      setScrambledImage(result);
    } catch (err) {
      console.error('Scramble error:', err);
      setError(typeof err === 'string' ? err : 'Failed to scramble image');
    } finally {
      setIsProcessing(false);
    }
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
      <div className="controls-panel">
        <h2>Image Scrambler</h2>
        
        <div className="file-upload-container">
          <label>Select Image</label>
          <input
            type="file"
            accept="image/*"
            onChange={handleImageUpload}
            className="file-input"
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

        <div className="button-group">
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

        {error && <div className="error-message">{error}</div>}
      </div>

      {/* Image Comparison Section */}
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
  );
}

export default SingleImage;