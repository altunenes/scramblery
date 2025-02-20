import React from 'react';

export interface BlockOptions {
  block_size: [number, number];
  interpolate_edges: boolean;
  padding_mode: 'Zero' | 'Reflect' | 'Wrap';
}

interface BlockControlsProps {
  options: BlockOptions;
  onChange: (options: BlockOptions) => void;
}

export function BlockControls({ options, onChange }: BlockControlsProps) {
  return (
    <div className="fourier-controls">
      <div className="control-group">
        <label>Block Size</label>
        <div className="flex gap-4">
          <div className="flex-1">
            <label className="text-sm text-gray-600">Width</label>
            <input
              type="number"
              min="8"
              max="128"
              step="8"
              value={options.block_size[0]}
              onChange={(e) => onChange({
                ...options,
                block_size: [parseInt(e.target.value), options.block_size[1]]
              })}
              className="w-full p-2 border rounded"
            />
          </div>
          <div className="flex-1">
            <label className="text-sm text-gray-600">Height</label>
            <input
              type="number"
              min="8"
              max="128"
              step="8"
              value={options.block_size[1]}
              onChange={(e) => onChange({
                ...options,
                block_size: [options.block_size[0], parseInt(e.target.value)]
              })}
              className="w-full p-2 border rounded"
            />
          </div>
        </div>
      </div>

      <div className="checkbox-group">
        <label>
          <input
            type="checkbox"
            checked={options.interpolate_edges}
            onChange={(e) => onChange({
              ...options,
              interpolate_edges: e.target.checked
            })}
          />
          Smooth Block Edges
        </label>
      </div>

      <div className="control-group">
        <label>Padding Mode</label>
        <select
          value={options.padding_mode}
          onChange={(e) => onChange({
            ...options,
            padding_mode: e.target.value as BlockOptions['padding_mode']
          })}
          className="select-input"
        >
          <option value="Zero">Zero Padding</option>
          <option value="Reflect">Reflect Padding</option>
          <option value="Wrap">Wrap Padding</option>
        </select>
      </div>
    </div>
  );
}