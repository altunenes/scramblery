### Prerequisites

Before running Scramblery, you need to install GStreamer:


1. Download and install GStreamer from the official website: https://gstreamer.freedesktop.org/download/#macos 
I built GStreamer 1.24.12 (see: https://github.com/altunenes/scramblery/blob/main/.github/workflows/release.yml#L81C1-L82C1), but I don't think the version matters, download the latest version.

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

**Q: Why don't you include GPU support for face detection?**

*A: Two reason. First, face detection model already very small. Second, creating different installers for each different onnx execution provider is rather pointless for such a simple face detection task in my opinion.*



### Ubuntu

Unfortunately, I couldn't test the app on Ubuntu, but it should work. I only suspect the path problem to finding the face detection model (on here: https://github.com/altunenes/scramblery/blob/c5c7692eacb6d0fd36b2e50de7718adfc8385d3c/face_detection/src/lib.rs#L52). I don't remember how the Tauri build deb handle the embedding paths. But it should be easy to fix if its not working. Please just open an issue/PR if you encounter any problem so we can fix it :)