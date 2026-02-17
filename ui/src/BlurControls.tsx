export interface BlurOptions {
  sigma: number;
}

interface BlurControlsProps {
  options: BlurOptions;
  onChange: (options: BlurOptions) => void;
}

export function BlurControls({ options, onChange }: BlurControlsProps) {
  return (
    <div className="blur-controls">
      <div className="control-group">
        <label>Blur Sigma</label>
        <div className="slider-row">
          <input
            type="range"
            min="0.5"
            max="25"
            step="0.5"
            value={options.sigma}
            onChange={(e) => onChange({
              ...options,
              sigma: parseFloat(e.target.value)
            })}
          />
          <span className="slider-value">{options.sigma.toFixed(1)}</span>
        </div>
      </div>
    </div>
  );
}
