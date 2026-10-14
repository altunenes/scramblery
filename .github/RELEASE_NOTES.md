- ONNX Runtime updated to 1.31 (ort 2.0.0-rc.14), with fixes for the WebGPU and CoreML providers.
- New builds: `-openvino` (Windows, Linux) for Intel GPUs and `-migraphx` (Linux) for AMD GPUs.
- Face boxes and outlines can differ by a fraction of a pixel from 4.0, so outputs of face scrambling may differ slightly from those of 4.0 with the same recipe.

CUDA builds need CUDA 13 and cuDNN 9, OpenVINO builds need Intel OpenVINO, and MIGraphX builds need ROCm. On Linux, use the `.deb` for GPU. While following motion, the app shows whether optical flow runs on the GPU or the CPU. See [INSTALL.md](https://github.com/altunenes/scramblery/blob/main/INSTALL.md).
