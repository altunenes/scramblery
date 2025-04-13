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
        <label>Blur Amount (Sigma)</label>
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
          className="w-full"
        />
        <div className="flex justify-between mt-1">
          <span>0.5</span>
          <span className="font-medium">{options.sigma.toFixed(1)}</span>
          <span>20</span>
        </div>
      </div>
    </div>
  );
}