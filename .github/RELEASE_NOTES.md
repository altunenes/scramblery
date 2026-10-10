Scramblery 4.0 is the biggest update so far. The app is now written entirely in Rust with a new interface, and most of the work went into the scientific side: what each method preserves, how results can be checked, and how they can be reproduced.

### New interface

The app moved from Tauri/React to [Slint](https://slint.dev), so there is no web stack anymore. Every setting has a short explanation next to it, and there is a light and a dark theme.

### Methods

- **Local phase scrambling (new):** randomises the phase within Gabor-like bands of scale and orientation while keeping the local amplitude. Unlike Fourier phase scrambling, contrast stays where it was in the image.
- **Fourier phase scrambling** now uses one phase field for all colour channels, so no false colours appear, and it keeps the mean of every channel. Frequency ranges can be given in cycles per pixel, per image or per degree of visual angle.
- **Faces** can be scrambled inside a box, an ellipse or the face outline (from facial landmarks), with soft edges.
- **Apertures:** show results through a circular or elliptical window on a uniform surround.
- **Linear light:** processing can optionally run on linear rather than gamma encoded values.

### Matching and normalization

- Match each scrambled image to its original, to fixed values or to the average of a reference set: mean luminance and RMS contrast, histograms or amplitude spectra.
- A new Normalize page equates these properties across a folder of images, using the methods of the SHINE toolbox (Willenbockel et al., 2010).
- The Analysis view compares original and result: luminance, contrast, histograms, amplitude spectrum and slope, clipping and SSIM.

### Reproducibility

- Every result records its seed, and saved images get a JSON file with the exact settings. Load it as a recipe to get the same result again, on any machine.
- Batch runs write a CSV report with statistics for each image, and can make several seeded versions per image or an intensity series for psychometric functions.
- 16-bit images stay 16-bit.

### Video

- Frames are converted exactly in their own colour space, so an unchanged frame comes back with identical luma. 10-bit sources give 10-bit output with the Lossless and High quality settings.
- **Spatiotemporal mode (new):** scrambles space and time together and keeps spatial and temporal frequency content.
- Follow motion now ignores unreliable optical flow and no longer draws edges of the original into the output.
- Independent frames are processed in parallel.

### Command line

`scramblery-cli` runs every method, the analysis and normalization from scripts. It is not in the installers yet; see BUILD.md to build it.

### Documentation

[docs/methods.md](https://github.com/altunenes/scramblery/blob/main/docs/methods.md) describes exactly what each method does and how every statistic is computed.

---

Grab the plain installer unless you specifically want GPU acceleration for optical flow. The `-cuda`, `-directml` and `-webgpu` builds only affect that, and they all fall back to CPU if the runtime isn't there. GStreamer still needs to be installed separately; see [INSTALL.md](https://github.com/altunenes/scramblery/blob/main/INSTALL.md).
