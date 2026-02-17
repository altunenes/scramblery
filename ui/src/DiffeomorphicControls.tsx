export interface DiffeomorphicOptions {
  max_distortion: number;
  n_steps: number;
  n_comp: number;
}

interface DiffeomorphicControlsProps {
  options: DiffeomorphicOptions;
  onChange: (options: DiffeomorphicOptions) => void;
}

export function DiffeomorphicControls({ options, onChange }: DiffeomorphicControlsProps) {
  return (
    <div className="fourier-controls">
      <div className="control-group">
        <label>Max Distortion (px)</label>
        <div className="slider-row">
          <input
            type="range"
            min="1"
            max="50"
            step="0.5"
            value={options.max_distortion}
            onChange={(e) => onChange({
              ...options,
              max_distortion: parseFloat(e.target.value)
            })}
          />
          <span className="slider-value">{options.max_distortion.toFixed(1)}</span>
        </div>
        <p className="info-note">
          RMS pixel displacement. Higher = more distortion. 5-10 is typical.
        </p>
      </div>

      <div className="control-group">
        <label>Warp Steps</label>
        <div className="slider-row">
          <input
            type="range"
            min="1"
            max="50"
            step="1"
            value={options.n_steps}
            onChange={(e) => onChange({
              ...options,
              n_steps: parseInt(e.target.value)
            })}
          />
          <span className="slider-value">{options.n_steps}</span>
        </div>
        <p className="info-note">
          More steps = smoother, more invertible warp. 10-20 is typical.
        </p>
      </div>

      <div className="control-group">
        <label>DCT Components</label>
        <div className="slider-row">
          <input
            type="range"
            min="2"
            max="20"
            step="1"
            value={options.n_comp}
            onChange={(e) => onChange({
              ...options,
              n_comp: parseInt(e.target.value)
            })}
          />
          <span className="slider-value">{options.n_comp}</span>
        </div>
        <p className="info-note">
          Spatial frequency of warp field. Low = broad warps, high = fine warps.
        </p>
      </div>

      <p className="info-note" style={{ marginTop: '0.5rem', borderTop: '1px solid #e0e0e0', paddingTop: '0.5rem' }}>
        Based on: Stojanoski, B., &amp; Cusack, R. (2014). Time to wave good-bye to phase scrambling: Creating controlled scrambled images using diffeomorphic transformations. <em>Journal of Vision</em>, 14(12):6, 1-16. doi:10.1167/14.12.6
      </p>
    </div>
  );
}
