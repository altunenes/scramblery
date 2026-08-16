### Video scrambling is much faster

Optical flow (the temporal coherence option) used to eat all your RAM on 1080p clips and often never finished a single frame. It now runs at roughly 1.4s per frame using about 2GB.

Your output resolution doesn't change. Only the motion estimation runs smaller internally; scrambling and warping still happen at full size.

### New: Output Fidelity setting

Video was always encoded at 4:2:0 chroma, which throws away most of the colour detail before the encoder even runs. That matters when the scramble pattern is your stimulus. You can now pick:

- **Fast (GPU, 4:2:0)**: the default, same as previous releases
- **High Quality (4:4:4)**: full colour, light compression
- **Lossless (4:4:4)**: use this for real stimulus material

Both 4:4:4 modes encode in software, so you get the same output on macOS, Windows and Linux. Before this, quality quietly depended on which encoder your machine happened to have.

### Audio is left alone now

Audio was being decoded and re-encoded on every run even though scrambling never touches it. AAC, MP3, AC3 and Opus tracks now get copied straight through.

### Fixes

- Progress bar could run past 100% (up to 190% on short clips) and look stuck when it was actually still working.
- Dev builds now compile the heavy image code optimised.

### New Windows build

There's a `-directml` installer now. It gives you GPU acceleration on any DirectX 12 card with nothing extra to install, unlike the CUDA builds.

The `-migraphx` build is gone. I can add this future if ONNX Runtime ship a MIGraphX binary

---

Grab the plain installer unless you specifically want GPU acceleration for optical flow. The `-cuda`, `-directml` and `-webgpu` builds only affect that, and they all fall back to CPU if the runtime isn't there.
