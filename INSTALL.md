### Prerequisites

Before running Scramblery, you need to install GStreamer:


1. Download and install GStreamer from the official website: https://gstreamer.freedesktop.org/download/
GStreamer is not bundled with Scramblery to keep the installer size small. Download the latest version — the exact version shouldn't matter.

Installing "runtime installer" should be enough, but if you want to develop and contribute to Scramblery with GStreamer, you can install "development installer" too.

2. Download Scramblery from the release page:
  https://github.com/altunenes/scramblery/releases

### Troubleshooting
- In macOS, you probably need to give permission to the app to run. For this please go to System Preferences -> Security & Privacy -> General -> Open Anyway

### Potential Questions

**Q: Why do I need GStreamer?**  

*A: GStreamer is a multimedia framework that allows you to process multimedia data. Scramblery uses GStreamer to process video files.*

**Q: Why don't you include GStreamer in the app?**  

*A: GStreamer is a large library and including it in the app would make the app size very large. Also, it would be hard to maintain the GStreamer version in the app.*

**Q: But can't we just embed the necessary dlls/libs?**

*A: Yes, we can. But installing Gstreamer is not a big deal like installing CUDA or something. It's just a few clicks and a few MBs. So, I think it's not worth the effort to embed the necessary dlls/libs also it would make the app size larger and harder to maintain.*

**Q: I see `-cuda` and `-migraphx` installers. Do I need those?**

*A: Only if you want GPU acceleration for optical flow. The default installers use CoreML (macOS) and DirectML (Windows) automatically. The CUDA/MIGraphX builds don't bundle those runtimes — you need to install CUDA or ROCm+MIGraphX yourself. If the runtime isn't found, it just falls back to CPU anyway.*



### Ubuntu

Unfortunately, I couldn't test the app on Ubuntu, but it should work. I only suspect the path problem to finding the face detection model (on here: https://github.com/altunenes/scramblery/blob/c5c7692eacb6d0fd36b2e50de7718adfc8385d3c/face_detection/src/lib.rs#L52). I don't remember how the Tauri build deb handle the embedding paths. But it should be easy to fix if its not working. Please just open an issue/PR if you encounter any problem so we can fix it :)