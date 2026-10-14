- ONNX Runtime updated to 1.31 (ort 2.0.0-rc.14), with fixes for the WebGPU and CoreML providers, and many improvements for cpu and gpu performance and gpu ep stability.
- New builds: `-openvino` (Windows, Linux) for Intel GPUs and `-migraphx` (Linux) for AMD GPUs.

CUDA builds need CUDA 13 and cuDNN 9, OpenVINO builds need Intel OpenVINO, and MIGraphX builds need ROCm. On Linux, use the `.deb` for GPU. While following motion, the app shows whether optical flow runs on the GPU or the CPU. See [INSTALL.md](https://github.com/altunenes/scramblery/blob/main/INSTALL.md).
