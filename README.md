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

- **Image Scrambling:** Various techniques—including advanced Fourier-based scrambling.
- **Video Scrambling:** Efficient frame processing via GStreamer's pipeline.
- **Batch Processing & Face Detection:** Process image directories and optionally target faces.
- **Face Detection:** Detect facial area and scramble facial area (options for: exclude/include bg).

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

## Citation

Altun, E. (2025). altunenes/scramblery: Scramblery v2.0.0 (v2.0.0). Zenodo. https://doi.org/10.5281/zenodo.7484576
