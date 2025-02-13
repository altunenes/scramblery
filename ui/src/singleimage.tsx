import React, { useState, ChangeEvent } from 'react';
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

function SingleImage() {
  const [selectedImage, setSelectedImage] = useState<SelectedImage | null>(null);
  const [scrambledImage, setScrambledImage] = useState<string | null>(null);
  const [intensity, setIntensity] = useState(50);
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

      // Get original image dimensions
      const img = new Image();
      img.src = URL.createObjectURL(file);
      await new Promise((resolve) => {
        img.onload = () => {
          setSelectedImage({
            data: base64String,
            name: file.name,
            originalSize: {
              width: img.width,
              height: img.height
            }
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
      const result = await invoke<string>('scramble_image', {
        imageData: selectedImage.data,
        options: {
          intensity: intensity / 100,
          seed: null
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

  const handleSaveScrambled = async () => {
    if (!scrambledImage || !selectedImage) return;

    try {
      const filePath = await save({
        filters: [{
          name: 'Image',
          extensions: ['png']
        }],
        defaultPath: selectedImage.name.replace(/\.[^/.]+$/, "") + '_scrambled.png'
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
    <div className="flex flex-col space-y-6 p-4 max-w-7xl mx-auto">
      <div className="space-y-4">
        <h2 className="text-2xl font-bold">Image Scrambler</h2>
        
        {/* Controls Section */}
        <div className="flex flex-col md:flex-row gap-4">
          {/* Upload and Options */}
          <div className="flex-1 space-y-4">
            <div>
              <input
                type="file"
                accept="image/*"
                onChange={handleImageUpload}
                className="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-semibold file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100"
              />
            </div>

            <div className="space-y-2">
              <label className="block text-sm font-medium text-gray-700">
                Scramble Intensity: {intensity}%
              </label>
              <input
                type="range"
                min="0"
                max="100"
                value={intensity}
                onChange={(e) => setIntensity(Number(e.target.value))}
                className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
              />
            </div>

            <div className="flex gap-2">
              <button
                onClick={handleScramble}
                disabled={!selectedImage || isProcessing}
                className="flex-1 px-4 py-2 bg-blue-500 text-white rounded-md disabled:bg-gray-300 disabled:cursor-not-allowed hover:bg-blue-600 transition-colors"
              >
                {isProcessing ? "Processing..." : "Scramble Image"}
              </button>

              {scrambledImage && (
                <button
                  onClick={handleSaveScrambled}
                  className="px-4 py-2 bg-green-500 text-white rounded-md hover:bg-green-600 transition-colors"
                >
                  Save Scrambled
                </button>
              )}
            </div>

            {error && (
              <div className="text-red-500 text-sm">
                {error}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Image Preview Section */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
        {selectedImage && (
          <div className="space-y-2">
            <h3 className="font-medium">Original Image</h3>
            <div className="relative aspect-video bg-gray-100 rounded-lg overflow-hidden">
              <img 
                src={`data:image/png;base64,${selectedImage.data}`}
                alt="Original"
                className="absolute w-full h-full object-contain"
              />
            </div>
          </div>
        )}
        {scrambledImage && (
          <div className="space-y-2">
            <h3 className="font-medium">Scrambled Image</h3>
            <div className="relative aspect-video bg-gray-100 rounded-lg overflow-hidden">
              <img 
                src={`data:image/png;base64,${scrambledImage}`}
                alt="Scrambled"
                className="absolute w-full h-full object-contain"
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default SingleImage;