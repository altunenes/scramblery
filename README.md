# Scramblery  
[![DOI](https://zenodo.org/badge/449034134.svg)](https://zenodo.org/badge/latestdoi/449034134) [![Release](https://github.com/altunenes/scramblery/actions/workflows/release.yml/badge.svg)](https://github.com/altunenes/scramblery/actions/workflows/release.yml)

---

<p align="center">
<img src="https://github.com/user-attachments/assets/3d81b541-8bb3-4414-9b77-501fe608041f" width="64" alt="App Icon">
</p>

Scramblery is a tool designed to scramble images and videos for scientific, privacy, or artistic purposes. It offers various scrambling techniques, including pixel shuffling and frequency domain manipulation using Fourier transforms. 
This is a "byproduct" of my research for my master's [thesis](https://www.tandfonline.com/doi/full/10.1080/13506285.2024.2415721) along with [butter2d](https://github.com/altunenes/butter2d) :-) 


---
*Preview of the main menu*

<p align="center">
  <img src="https://github.com/user-attachments/assets/1137f7f2-9169-4e4c-a14d-5dddaf5a3647" width="400" alt="Main Menu Preview"/>
</p>

---

## Features

- **Image Scrambling:** Pixel, Fourier phase, block, Gaussian blur and diffeomorphic scrambling (Stojanoski & Cusack, 2014).
- **Spatial-Frequency Control:** Scramble (or filter) a low-pass, high-pass or band-pass range given in cycles per pixel, cycles per image or cycles per degree.
- **Analysis:** Compare original and scrambled images: mean luminance, RMS contrast, histograms, rotationally averaged amplitude spectrum and its slope, clipping and SSIM.
- **Normalization:** Equate mean luminance and contrast, histograms or amplitude spectra across a set of images, using the methods of the SHINE toolbox (Willenbockel et al., 2010).
- **Matching:** Match each scrambled image to its original, to fixed values or to a set average, live in the preview and in batch and video processing.
- **Stimulus Sets:** Batch processing of folders, several differently seeded scrambles per image, and intensity series that vary only the scrambling strength, e.g. for psychometric functions.
- **Faces and Apertures:** Restrict scrambling to detected faces (face outline from facial landmarks, ellipse or box, with soft edges), keeping or removing the background, and show stimuli through circular or elliptical windows on a uniform surround.
- **Reproducibility:** Every result records its seed; saved images get a JSON file with the exact settings, batch runs write a CSV report with per-image statistics, and settings can be saved and loaded as recipes.
- **Bit Depth and Linear Light:** 16-bit images are processed and saved at 16 bits, and processing can optionally run in linear light rather than on gamma-encoded values.
- **Video Scrambling:** Frame-wise scrambling with a fixed or changing pattern, scrambling that follows the original motion (SEA-RAFT optical flow), and spatiotemporal phase scrambling that preserves spatial and temporal frequency content. Video is processed losslessly in its own colour space, at up to 10 bits, and in parallel.
- **Command Line:** `scramblery-cli` runs every method, the analysis and normalization from scripts. See [BUILD.md](BUILD.md).

How each method and statistic is defined is described in [docs/methods.md](docs/methods.md).

## Examples

---

<div align="center">

| Video | GIF Preview |
|:-----:|:-----------:|
| [Video](https://github.com/user-attachments/assets/affb7333-231d-4773-bfa4-44c4c05fd815) <br><sub>Block scramble-only facial area exclude bg</sub> | <img src="https://github.com/user-attachments/assets/a6ca6476-e940-46c7-a9f4-b549e6bfa503" width="200" alt="block" /> |
| [Video](https://github.com/user-attachments/assets/0a57b9a9-859d-4a4f-96cf-a4fe39f98637) <br><sub>Pixel scramble-only facial area exclude bg</sub> | <img src="https://github.com/user-attachments/assets/ebdcf320-9e1a-4ebd-b5a2-64b29c00146d" width="200" alt="exc_bg" /> |
| [Video 1](https://github.com/user-attachments/assets/37fd72dc-e575-4de0-b910-94a42e81f0b2) <br>[Video 2](https://github.com/user-attachments/assets/a0f47a62-7867-4fb1-ae6f-bff893225d47) <br><sub>Fourier Phase: Colorful/Gray options</sub> | <img src="https://github.com/user-attachments/assets/6d011ce0-7cfc-429d-8d2e-93f3c5f19c5d" width="200" alt="fft" /> |

</div>


---

## Installation

For installation instructions, please refer to the [INSTALLATION GUIDE](https://github.com/altunenes/scramblery/blob/main/INSTALL.md).

---

## Acknowledgements

Scramblery uses the following methods. If you use these features in your research, please cite the original authors also:

**Diffeomorphic Scrambling:**

> Stojanoski, B., & Cusack, R. (2014). Time to wave good-bye to phase scrambling: Creating controlled scrambled images using diffeomorphic transformations. *Journal of Vision*, 14(12):6, 1–16. doi:[10.1167/14.12.6](https://doi.org/10.1167/14.12.6)

**Normalization:**

> Willenbockel, V., Sadr, J., Fiset, D., Horne, G. O., Gosselin, F., & Tanaka, J. W. (2010). Controlling low-level image properties: The SHINE toolbox. *Behavior Research Methods*, 42(3), 671–684. doi:[10.3758/BRM.42.3.671](https://doi.org/10.3758/BRM.42.3.671)

**SEA-RAFT Optical Flow (Follow motion):**

> Wang, Y., Lipson, L., & Deng, J. (2024). SEA-RAFT: Simple, Efficient, Accurate RAFT for Optical Flow. *European Conference on Computer Vision (ECCV)*. Princeton Vision & Learning Lab.

---

## Citation

Altun, E. (2025). altunenes/scramblery: Scramblery v2.0.0 (v2.0.0). Zenodo. https://doi.org/10.5281/zenodo.7484576
