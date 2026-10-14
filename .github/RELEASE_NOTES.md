- Follow motion has been rewritten. The scramble now moves with the content as an advected texture: it no longer blurs over time, no longer traces facial features (eyes, mouth, outlines) where motion estimates fail, and partial scrambles (e.g. a Fourier frequency range) no longer leave trails of earlier frames. "Keyframe interval" is now "Refresh interval" (default 12 frames), and "Keyframe blend" is no longer needed. See [docs/methods.md](https://github.com/altunenes/scramblery/blob/main/docs/methods.md).
- Longer descriptions in the app are no longer cut off.

CUDA builds need CUDA 13 and cuDNN 9, OpenVINO builds need Intel OpenVINO, and MIGraphX builds need ROCm. On Linux, use the `.deb` for GPU. See [INSTALL.md](https://github.com/altunenes/scramblery/blob/main/INSTALL.md).
