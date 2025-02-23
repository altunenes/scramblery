 # Scramblery ![32x32](https://github.com/user-attachments/assets/77abf531-2e86-475a-8811-e81085e904ca)

[![DOI](https://zenodo.org/badge/449034134.svg)](https://zenodo.org/badge/latestdoi/449034134)


**Please note: This project is currently under active development and is not yet feature-complete or production-ready.** Expect changes and potential instability.


**Scramblery** is a simple tool designed to scramble images and videos for scientific, privacy or artistic purposes. It offers various scrambling techniques, including pixel shuffling and frequency domain manipulation using Fourier transforms.  This application is built using Rust for the backend to ensure performance and safety. 

Citation

`Altun, E. (2025). altunenes/scramblery: Scramblery v2.0.0 (v2.0.0). Zenodo. https://doi.org/10.5281/zenodo.7484576`

## Features (Currently Implemented or Planned)

*   **Image Scrambling:**
    *   **Pixel Scrambling:** Shuffles pixels within an image based on intensity.
    *   **Fourier Scrambling:**  Scrambles images in the frequency domain using Fourier transforms, offering different levels of obfuscation.
        *   Phase scrambling.
        *   Frequency range filtering (Low-pass, High-pass, Band-pass).
    *   **Grayscale Processing:** Option to process images in grayscale for Fourier scrambling.
    *   **Face Detection Integration:** Option to apply scrambling selectively to detected faces or the background, using a pre-trained ONNX model.
        *   Face region expansion for better coverage.
        *   Background modes: Include (scramble faces, keep background), Exclude (scramble faces only, output scrambled faces on black background).
*   **Batch Image Processing:** Process entire directories of images with the desired settings.
*   **Video Scrambling:** Apply scrambling to video frames.
    *   Video processing pipeline using GStreamer for efficient decoding and encoding.


Example for a video: 
Pixel scramble-only facial area keep bg

https://github.com/user-attachments/assets/75f6a369-0a81-4542-8c65-632aa70be8a2

Pixel scramble-only facial area exclude bg

https://github.com/user-attachments/assets/37fd72dc-e575-4de0-b910-94a42e81f0b2

Phase-scramble only facial area keep bg (you can also exclude bg like pixel scramble video)

https://github.com/user-attachments/assets/7e3921a5-b7c9-4159-8991-7c1d6d496840

