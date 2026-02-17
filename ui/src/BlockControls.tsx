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
        <div className="block-size-row">
          <div>
            <label>Width</label>
            <input
              type="number"
              min="4"
              max="128"
              step="4"
              value={options.block_size[0]}
              onChange={(e) => onChange({
                ...options,
                block_size: [parseInt(e.target.value), options.block_size[1]]
              })}
            />
          </div>
          <div>
            <label>Height</label>
            <input
              type="number"
              min="4"
              max="128"
              step="4"
              value={options.block_size[1]}
              onChange={(e) => onChange({
                ...options,
                block_size: [options.block_size[0], parseInt(e.target.value)]
              })}
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
      <p className="info-note">
        Blends pixels at block boundaries. Disabling preserves exact original channel values.
      </p>

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
