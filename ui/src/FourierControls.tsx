interface FourierOptions {
  frequency_range: FrequencyRange;
  phase_scramble: boolean;
  magnitude_scramble: boolean;
  padding_mode: PaddingMode;
  intensity: number;
  grayscale: boolean;
}

type FrequencyRange =
  | 'All'
  | { HighPass: { cutoff: number } }
  | { LowPass: { cutoff: number } }
  | { BandPass: { low: number, high: number } };

type PaddingMode = 'Zero' | 'Reflect' | 'Wrap';

export function FourierControls({
  options,
  onChange
}: {
  options: FourierOptions;
  onChange: (options: FourierOptions) => void;
}) {
  const getCurrentRangeType = (range: FrequencyRange): string => {
    if (range === 'All') return 'All';
    return Object.keys(range)[0];
  };

  const getCutoffValue = (range: FrequencyRange): number => {
    if (range === 'All') return 0;
    if ('HighPass' in range) return range.HighPass.cutoff;
    if ('LowPass' in range) return range.LowPass.cutoff;
    return 0;
  };

  return (
    <div className="fourier-controls">
      <div className="control-group">
        <label>Frequency Range</label>
        <select
          value={getCurrentRangeType(options.frequency_range)}
          onChange={(e) => {
            const type = e.target.value;
            let range: FrequencyRange;

            switch(type) {
              case 'All':
                range = 'All';
                break;
              case 'HighPass':
                range = { HighPass: { cutoff: 0.5 } };
                break;
              case 'LowPass':
                range = { LowPass: { cutoff: 0.5 } };
                break;
              case 'BandPass':
                range = { BandPass: { low: 0.3, high: 0.7 } };
                break;
              default:
                return;
            }

            onChange({
              ...options,
              frequency_range: range
            });
          }}
          className="select-input"
        >
          <option value="All">All Frequencies</option>
          <option value="HighPass">High Pass Filter</option>
          <option value="LowPass">Low Pass Filter</option>
          <option value="BandPass">Band Pass Filter</option>
        </select>
      </div>

      {options.frequency_range !== 'All' && (
        <div className="control-group">
          {'BandPass' in options.frequency_range ? (
            <>
              <label>Low Cutoff</label>
              <div className="slider-row">
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={(options.frequency_range as { BandPass: { low: number } }).BandPass.low}
                  onChange={(e) => {
                    const newLow = Number(e.target.value);
                    const currentHigh = (options.frequency_range as { BandPass: { high: number } }).BandPass.high;
                    onChange({
                      ...options,
                      frequency_range: {
                        BandPass: {
                          low: Math.min(newLow, currentHigh - 0.01),
                          high: currentHigh
                        }
                      }
                    });
                  }}
                />
                <span className="slider-value">{(options.frequency_range as { BandPass: { low: number } }).BandPass.low.toFixed(2)}</span>
              </div>
              <label>High Cutoff</label>
              <div className="slider-row">
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={(options.frequency_range as { BandPass: { high: number } }).BandPass.high}
                  onChange={(e) => {
                    const newHigh = Number(e.target.value);
                    const currentLow = (options.frequency_range as { BandPass: { low: number } }).BandPass.low;
                    onChange({
                      ...options,
                      frequency_range: {
                        BandPass: {
                          low: currentLow,
                          high: Math.max(newHigh, currentLow + 0.01)
                        }
                      }
                    });
                  }}
                />
                <span className="slider-value">{(options.frequency_range as { BandPass: { high: number } }).BandPass.high.toFixed(2)}</span>
              </div>
            </>
          ) : (
            <>
              <label>Cutoff Frequency</label>
              <div className="slider-row">
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={getCutoffValue(options.frequency_range)}
                  onChange={(e) => {
                    const value = Number(e.target.value);
                    const currentType = getCurrentRangeType(options.frequency_range);
                    onChange({
                      ...options,
                      frequency_range: currentType === 'HighPass'
                        ? { HighPass: { cutoff: value } }
                        : { LowPass: { cutoff: value } }
                    });
                  }}
                />
                <span className="slider-value">{getCutoffValue(options.frequency_range).toFixed(2)}</span>
              </div>
            </>
          )}
        </div>
      )}

      <div className="checkbox-group">
        <label>
          <input
            type="checkbox"
            checked={options.phase_scramble}
            onChange={(e) => onChange({
              ...options,
              phase_scramble: e.target.checked
            })}
          />
          Scramble Phases
        </label>
      </div>

      <div className="checkbox-group">
        <label>
          <input
            type="checkbox"
            checked={options.magnitude_scramble}
            onChange={(e) => onChange({
              ...options,
              magnitude_scramble: e.target.checked
            })}
          />
          Scramble Magnitudes
        </label>
      </div>

      <div className="control-group">
        <label>Padding Mode</label>
        <select
          value={options.padding_mode}
          onChange={(e) => onChange({
            ...options,
            padding_mode: e.target.value as PaddingMode
          })}
          className="select-input"
        >
          <option value="Zero">Zero Padding</option>
          <option value="Reflect">Reflect Padding</option>
          <option value="Wrap">Wrap Padding</option>
        </select>
      </div>

      <div className="checkbox-group">
        <label>
          <input
            type="checkbox"
            checked={options.grayscale}
            onChange={(e) => onChange({
              ...options,
              grayscale: e.target.checked
            })}
          />
          Process as Grayscale
        </label>
      </div>
    </div>
  );
}

export type { FourierOptions, FrequencyRange, PaddingMode };
