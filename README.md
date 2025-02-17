 # Scramblery
[![DOI](https://zenodo.org/badge/449034134.svg)](https://zenodo.org/badge/latestdoi/449034134)
![sacasc](https://user-images.githubusercontent.com/54986652/227797464-3fc1fc88-a31b-4244-b99f-df0f77a6e282.png)


**Please note: This project is currently under active development and is not yet feature-complete or production-ready.** Expect changes and potential instability.


**Scramblery** is a tool designed to scramble images and videos for scientific, privacy or artistic purposes. It offers various scrambling techniques, including pixel shuffling and frequency domain manipulation using Fourier transforms.  This application is built using Rust for the backend to ensure performance and safety. 

Image scrambling plays a crucial role in psychology experiments, enabling researchers to manipulate visual stimuli while maintaining control over certain visual aspects.

Citation

`Altun, E. (2023). altunenes/scramblery: 1.2.5 (1.2.5). Zenodo. https://doi.org/10.5281/zenodo.10028991`

## Features (Currently Implemented or Planned)

*   **Image Scrambling:**
    *   **Pixel Scrambling:** Shuffles pixels within an image based on intensity.
    *   **Fourier Scrambling:**  Scrambles images in the frequency domain using Fourier transforms, offering different levels of obfuscation.
        *   Phase scrambling.
        *   Frequency range filtering (Low-pass, High-pass, Band-pass).
        *   Padding modes for FFT processing.
    *   **Grayscale Processing:** Option to process images in grayscale for Fourier scrambling.
    *   **Face Detection Integration:** Option to apply scrambling selectively to detected faces or the background, using a pre-trained ONNX model.
        *   Adjustable face detection confidence threshold.
        *   Face region expansion for better coverage.
        *   Background modes: Include (scramble faces, keep background), Exclude (scramble faces only, output scrambled faces on black background).
*   **Batch Image Processing:** Process entire directories of images with the same settings.
    *   Progress indication for batch processing.
    *   Error handling and reporting for individual image processing within a batch.
*   **Video Scrambling:** Apply scrambling to video frames.
    *   Video processing pipeline using GStreamer for efficient decoding and encoding.
