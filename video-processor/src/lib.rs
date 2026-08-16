use anyhow::Result;
use gstreamer as gst;
use gstreamer_video as gst_video;
use gstreamer_app as gst_app;
use std::path::Path;
use log::{info, error, debug};
use gst::prelude::*;
use gstreamer_pbutils::Discoverer;
pub use gst_video::video_frame::{VideoFrame, Readable, Writable, VideoFrameExt};
use gstreamer_pbutils::prelude::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum VideoFidelity {
    /// Lossless H.264 at full 4:4:4 chroma: decoded frames match the processed
    /// ones up to RGB<->YUV rounding
    Lossless,
    /// Full 4:4:4 chroma with mild quantisation
    HighQuality,
    /// HW encoder where available, 4:2:0
    #[default]
    Fast,
}

fn make_faithful_encoder(quantizer: u32) -> Result<(gst::Element, &'static str)> {
    let enc = gst::ElementFactory::make("x264enc")
        .property_from_str("pass", "quant")
        .property("quantizer", quantizer)
        .property_from_str("speed-preset", "superfast")
        .property("key-int-max", 25u32)
        .build()?;
    info!(
        "Using x264enc software encoder, 4:4:4 chroma, constant quantizer {} ({})",
        quantizer,
        if quantizer == 0 { "lossless" } else { "visually lossless" }
    );
    Ok((enc, "Y444"))
}

fn make_h264_encoder(fidelity: VideoFidelity) -> Result<(gst::Element, &'static str)> {
    match fidelity {
        VideoFidelity::Lossless => return make_faithful_encoder(0),
        VideoFidelity::HighQuality => return make_faithful_encoder(18),
        VideoFidelity::Fast => {}
    }

    if let Ok(enc) = gst::ElementFactory::make("vtenc_h264")
        .property("bitrate", 2048u32) 
        .property("max-keyframe-interval", 25i32)
        .property("realtime", true)
        .property("allow-frame-reordering", false)
        .build()
    {
        info!("Using VideoToolbox H.264 encoder (hardware)");
        return Ok((enc, "NV12"));
    }

    if let Ok(enc) = gst::ElementFactory::make("mfh264enc")
        .property("bitrate", 2048u32)
        .property("max-keyframe-interval", 25u32)
        .build()
    {
        info!("Using Media Foundation H.264 encoder (hardware)");
        return Ok((enc, "NV12"));
    }

    if let Ok(enc) = gst::ElementFactory::make("nvh264enc")
        .property("bitrate", 2048u32)
        .property("gop-size", 25u32)
        .property_from_str("preset", "low-latency-hp")
        .property_from_str("rc-mode", "cbr")
        .build()
    {
        info!("Using NVENC H.264 encoder (hardware)");
        return Ok((enc, "NV12"));
    }

    if let Ok(enc) = gst::ElementFactory::make("vaapih264enc")
        .property("bitrate", 2048u32)
        .property("keyframe-period", 25u32)
        .build()
    {
        info!("Using VA-API H.264 encoder (hardware)");
        return Ok((enc, "NV12"));
    }

    let enc = gst::ElementFactory::make("x264enc")
        .property_from_str("speed-preset", "ultrafast")
        .property("bitrate", 2048u32)
        .property("key-int-max", 25u32)
        .property_from_str("tune", "zerolatency")
        .build()?;
    info!("Using x264enc software encoder (CPU fallback)");
    Ok((enc, "I420"))
}


fn audio_passthrough_parser(structure: &gst::StructureRef) -> Option<&'static str> {
    match structure.name().as_str() {
        "audio/mpeg" => match structure.get::<i32>("mpegversion") {
            Ok(4) | Ok(2) => Some("aacparse"),
            Ok(1) => Some("mpegaudioparse"),
            _ => None,
        },
        "audio/x-ac3" | "audio/x-eac3" => Some("ac3parse"),
        "audio/x-opus" => Some("opusparse"),
        _ => None,
    }
}

pub struct VideoProcessor {
    pipeline: gst::Pipeline,
}

impl VideoProcessor {
    pub fn new() -> Result<Self> {
        info!("Initializing GStreamer");
        gst::init()?;

        let registry = gst::Registry::get();
        let hw_rank = gst::Rank::PRIMARY + 2;

        let hw_decoders = [
            "vtdec_hw", "vtdec",
            "nvh264dec", "nvh264sldec", "nvh265dec", "nvh265sldec", "nvav1dec",
            "vaapih264dec", "vaapih265dec", "vampeg2dec", "vaapiav1dec",
            "d3d11h264dec", "d3d11h265dec", "d3d11av1dec",
            "d3d12h264dec", "d3d12h265dec",
        ];

        for name in hw_decoders {
            if let Some(feature) = registry.lookup_feature(name) {
                feature.set_rank(hw_rank);
                info!("Boosted {} decoder rank for hardware decoding", name);
            }
        }

        let pipeline = gst::Pipeline::new();
        info!("GStreamer pipeline created");
        Ok(Self { pipeline })
    }

    pub fn process_video<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
        fidelity: VideoFidelity,
        frame_callback: impl Fn(&mut VideoFrame<Writable>) -> Result<()> + Send + 'static,
        progress_callback: impl Fn(f32) + Send + Sync + 'static,
    ) -> Result<()> {
        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();
        info!("Setting up video processing pipeline");
        info!("Input path: {:?}", input_path);
        info!("Output path: {:?}", output_path);

        let src = gst::ElementFactory::make("filesrc")
            .property("location", input_path.to_str().unwrap())
            .build()?;

        let decodebin = gst::ElementFactory::make("decodebin").build()?;
        let videoconvert1 = gst::ElementFactory::make("videoconvert").build()?;
        let videoscale1 = gst::ElementFactory::make("videoscale").build()?;
        let queue1 = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 2u32)
            .build()?;
        let appsink = gst::ElementFactory::make("appsink").build()?;

        let queue2 = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 0u32)
            .property("max-size-time", 0u64)
            .property("max-size-bytes", 0u32)
            .build()?;
        let appsrc = gst::ElementFactory::make("appsrc").build()?;
        let videoconvert2 = gst::ElementFactory::make("videoconvert").build()?;
        let videoscale2 = gst::ElementFactory::make("videoscale").build()?;
        let (encoder, caps_format) = make_h264_encoder(fidelity)?;

        let h264parse = gst::ElementFactory::make("h264parse").build()?;
        let queue3 = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 0u32)
            .property("max-size-time", 0u64)
            .property("max-size-bytes", 0u32)
            .build()?;
        let muxer = gst::ElementFactory::make("mp4mux").build()?;
        let sink = gst::ElementFactory::make("filesink")
            .property("location", output_path.to_str().unwrap())
            .property("sync", false)
            .property("async", false)
            .build()?;
        let capsfilter = gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .field("format", caps_format)
                .build(),
        )
        .build()?;

        let appsrc = appsrc.dynamic_cast::<gst_app::AppSrc>().unwrap();
        appsrc.set_format(gst::Format::Time);
        appsrc.set_property("is-live", false);
        appsrc.set_property("do-timestamp", true);
        appsrc.set_property("block", true);
        appsrc.set_property("format", gst::Format::Time);

        let appsink = appsink.dynamic_cast::<gst_app::AppSink>().unwrap();
        appsink.set_property("sync", false);
        appsink.set_property("drop", false);
        appsink.set_property("max-buffers", 0u32);

        // Add only video elements to the pipeline (audio added dynamically if present)
        self.pipeline.add_many(&[
            &src,
            &decodebin,
            &videoconvert1,
            &videoscale1,
            &queue1,
            &appsink.upcast_ref(),
            &appsrc.upcast_ref(),
            &queue2,
            &videoconvert2,
            &videoscale2,
            &capsfilter,
            &encoder,
            &h264parse,
            &queue3,
            &muxer,
            &sink,
        ])?;

        gst::Element::link_many(&[&videoconvert1, &videoscale1, &queue1, &appsink.upcast_ref()])?;
        gst::Element::link_many(&[
            &appsrc.upcast_ref(),
            &queue2,
            &videoconvert2,
            &videoscale2,
            &capsfilter,
            &encoder,
            &h264parse,
            &queue3,
            &muxer,
            &sink,
        ])?;

        gst::Element::link_many(&[&src, &decodebin])?;

        // Set caps for appsink
        appsink.set_caps(Some(&gst::Caps::builder("video/x-raw")
            .field("format", gst_video::VideoFormat::Rgba.to_str())
            .build()));

        let mut video_info: Option<gst_video::VideoInfo> = None;
        let appsrc_weak = appsrc.downgrade();
        let frame_count = std::sync::atomic::AtomicUsize::new(0);
        let total_frames = {
            let mut frames = 0;
            let timeout = gst::ClockTime::from_seconds(5);
            match Discoverer::new(timeout) {
                Ok(discoverer) => {
                    let path_str = input_path.to_str().unwrap();
                    let uri = if path_str.starts_with("file://") {
                        path_str.to_string()
                    } else {
                        let absolute_path = std::fs::canonicalize(&input_path)
                            .unwrap_or(input_path.to_path_buf());
                        let path_str = absolute_path.to_str().unwrap()
                            .trim_start_matches("\\?\\")
                            .replace("\\", "/");
                        format!("file:///{}", path_str)
                    };

                    info!("Trying to discover URI: {}", uri);
                    match discoverer.discover_uri(&uri) {
                        Ok(info) => {
                            if let Some(video_info) = info.video_streams().get(0) {
                                if let Some(caps) = video_info.caps() {
                                    if let Some(s) = caps.structure(0) {
                                        if let Ok(framerate) = s.get::<gst::Fraction>("framerate") {
                                            let duration = info.duration();
                                            let duration_secs = duration.unwrap_or(gst::ClockTime::ZERO).nseconds() as f64 / 1e9;
                                            let fps = framerate.numer() as f64 / framerate.denom() as f64;
                                            frames = (duration_secs * fps).round() as usize;
                                            info!("Got exact framerate {}/{} fps, duration {:.3}s, calculated frames: {}",
                                                 framerate.numer(), framerate.denom(),
                                                 duration_secs, frames);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            info!("Failed to discover video: {}", e);
                        }
                    }
                }
                Err(e) => {
                    info!("Failed to create discoverer: {}", e);
                }
            }

            if frames == 0 {
                info!("Falling back to pipeline method");
                let input_str = input_path.to_str().unwrap().replace("\\", "\\\\");
                let pipeline_str = format!(
                    "filesrc location=\"{}\" ! decodebin ! video/x-raw ! fakesink",
                    input_str
                );

                match gst::parse::launch(&pipeline_str) {
                    Ok(temp_element) => {
                        let temp_pipeline = temp_element.dynamic_cast::<gst::Pipeline>().unwrap();

                        if temp_pipeline.set_state(gst::State::Paused).is_ok() {
                            std::thread::sleep(std::time::Duration::from_secs(1));

                            if let Some(duration) = temp_pipeline.query_duration::<gst::ClockTime>() {
                                if let Some(sink) = temp_pipeline.by_name("fakesink0") {
                                    if let Some(sink_pad) = sink.static_pad("sink") {
                                        if let Some(caps) = sink_pad.current_caps() {
                                            if let Some(s) = caps.structure(0) {
                                                if let Ok(framerate) = s.get::<gst::Fraction>("framerate") {
                                                    let duration_secs = duration.nseconds() as f64 / 1e9;
                                                    frames = (duration_secs *
                                                            (framerate.numer() as f64 /
                                                             framerate.denom() as f64)).round() as usize;
                                                    info!("Got framerate from pipeline: {}/{} fps",
                                                         framerate.numer(), framerate.denom());
                                                }
                                            }
                                        }
                                    }
                                }
                                if frames == 0 {
                                    let duration_secs = duration.nseconds() as f64 / 1e9;
                                    frames = (duration_secs * 30.0).round() as usize;
                                    info!("Using estimated 30fps - duration: {:.3}s, frames: {}",
                                         duration_secs, frames);
                                }
                            }

                            let _ = temp_pipeline.set_state(gst::State::Null);
                        }
                    }
                    Err(e) => {
                        info!("Failed to create pipeline: {}", e);
                    }
                }
            }
            if frames == 0 {
                frames = 1000;
                info!("Using default frame count: {}", frames);
            }

            frames
        };

        info!("Final total frames: {}", total_frames);
        let appsrc_weak_cb = appsrc_weak.clone();
        let appsrc_weak_eos = appsrc_weak.clone();

        let progress_callback = std::sync::Arc::new(progress_callback);
        let progress_callback_sample = progress_callback.clone();
        let progress_callback_eos = progress_callback.clone();

        // Setup callbacks
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let count = frame_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    debug!("Processing frame {}", count);

                    if total_frames > 0 {
                        let progress = ((count as f32 / total_frames as f32) * 100.0).min(99.9);
                        progress_callback_sample(progress);
                    }

                    let sample = appsink.pull_sample().map_err(|_| {
                        debug!("No more samples, sending EOS");
                        if let Some(appsrc) = appsrc_weak_cb.upgrade() {
                            let _ = appsrc.end_of_stream();
                        }
                        gst::FlowError::Eos
                    })?;

                    let buffer = sample.buffer().ok_or_else(|| {
                        error!("No buffer in sample");
                        gst::FlowError::Error
                    })?;

                    let caps = sample.caps().ok_or_else(|| {
                        error!("No caps in sample");
                        gst::FlowError::Error
                    })?;

                    if video_info.is_none() {
                        let info = gst_video::VideoInfo::from_caps(caps).map_err(|_| {
                            error!("Failed to get video info from caps");
                            gst::FlowError::Error
                        })?;

                        if let Some(appsrc) = appsrc_weak_cb.upgrade() {
                            appsrc.set_caps(Some(&caps.to_owned()));
                            video_info = Some(info.clone());
                        }
                    }

                    let info = video_info.as_ref().unwrap();
                    let buffer = buffer.to_owned();
                    let mut frame = gst_video::VideoFrame::from_buffer_writable(buffer, info)
                        .map_err(|_| {
                            error!("Failed to create writable frame");
                            gst::FlowError::Error
                        })?;

                    if let Err(e) = frame_callback(&mut frame) {
                        error!("Frame callback error: {}", e);
                        return Err(gst::FlowError::Error);
                    }

                    if let Some(appsrc) = appsrc_weak_cb.upgrade() {
                        appsrc.push_buffer(frame.into_buffer())
                            .map_err(|_| gst::FlowError::Error)?;
                    }

                    debug!("Frame {} processed successfully", count);
                    Ok(gst::FlowSuccess::Ok)
                })
                .eos(move |_appsink| {
                    debug!("AppSink received EOS");
                    if let Some(appsrc) = appsrc_weak_eos.upgrade() {
                        let _ = appsrc.end_of_stream();
                    }
                    progress_callback_eos(100.0);
                })
                .build(),
        );
        decodebin.connect("autoplug-continue", false, |values| {
            let caps = match values[2].get::<gst::Caps>() {
                Ok(caps) => caps,
                Err(_) => return Some(true.to_value()),
            };
            let keep_decoding = match caps.structure(0) {
                Some(s) => audio_passthrough_parser(s).is_none(),
                None => true,
            };
            if !keep_decoding {
                info!("Audio stream is MP4-compatible, copying it instead of re-encoding");
            }
            Some(keep_decoding.to_value())
        });

        // Dynamically link audio/video from decodebin.
        let videoconvert1_weak = videoconvert1.downgrade();
        let pipeline_weak = self.pipeline.downgrade();
        let muxer_weak = muxer.downgrade();
        decodebin.connect_pad_added(move |_, pad| {
            let caps = pad.current_caps().unwrap();
            let structure = caps.structure(0).unwrap();
            let name = structure.name();
            debug!("New pad added with caps: {:?}", name);

            if name.starts_with("video/") {
                if let Some(videoconvert) = videoconvert1_weak.upgrade() {
                    let sink_pad = videoconvert.static_pad("sink").unwrap();
                    if let Err(e) = pad.link(&sink_pad) {
                        error!("Failed to link decoder to converter: {}", e);
                    } else {
                        info!("Linked decoder to video converter successfully");
                    }
                }
            } else if name.starts_with("audio/") {
                // Dynamically create and add audio elements only when audio is present
                if let (Some(pipeline), Some(muxer)) = (pipeline_weak.upgrade(), muxer_weak.upgrade()) {
                    let result = (|| -> std::result::Result<(), Box<dyn std::error::Error>> {
                        let queue_audio = gst::ElementFactory::make("queue")
                            .property("max-size-buffers", 0u32)
                            .property("max-size-time", 0u64)
                            .property("max-size-bytes", 0u32)
                            .build()?;
                        let mut chain: Vec<gst::Element> = vec![queue_audio.clone()];
                        match audio_passthrough_parser(structure) {
                            Some(parser) => {
                                chain.push(gst::ElementFactory::make(parser).build()?);
                                info!("Copying audio stream unchanged via {}", parser);
                            }
                            None => {
                                chain.push(gst::ElementFactory::make("audioconvert").build()?);
                                chain.push(gst::ElementFactory::make("audioresample").build()?);
                                chain.push(gst::ElementFactory::make("avenc_aac").build()?);
                                chain.push(gst::ElementFactory::make("aacparse").build()?);
                                info!("Audio codec {} cannot be stored in MP4, re-encoding to AAC", name);
                            }
                        }

                        let refs: Vec<&gst::Element> = chain.iter().collect();
                        pipeline.add_many(&refs)?;
                        gst::Element::link_many(&refs)?;
                        chain
                            .last()
                            .expect("audio chain is never empty")
                            .link(&muxer)
                            .map_err(|e| format!("Failed to link audio to muxer: {}", e))?;

                        // Sync state with parent pipeline
                        for element in &chain {
                            element.sync_state_with_parent()?;
                        }

                        let sink_pad = queue_audio.static_pad("sink").unwrap();
                        pad.link(&sink_pad).map_err(|e| format!("Failed to link audio: {}", e))?;

                        info!("Linked audio chain successfully");
                        Ok(())
                    })();

                    if let Err(e) = result {
                        error!("Failed to set up audio chain: {}", e);
                    }
                }
            }
        });

        // Start the pipeline
        info!("Starting pipeline");
        self.pipeline.set_state(gst::State::Playing)?;

        // Wait for completion or error
        info!("Waiting for pipeline to finish");
        let bus = self.pipeline.bus().unwrap();

        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            match msg.view() {
                gst::MessageView::Eos(..) => {
                    info!("Pipeline EOS reached");
                    break;
                },
                gst::MessageView::Error(err) => {
                    error!("Pipeline error: {} ({:?})", err.error(), err.debug());
                    self.pipeline.set_state(gst::State::Null)?;
                    return Err(anyhow::anyhow!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    ));
                }
                gst::MessageView::StateChanged(state) => {
                    debug!("State changed: {:?}", state);
                }
                _ => (),
            }
        }

        // Clean up - set to NULL state
        info!("Cleaning up pipeline");
        self.pipeline.set_state(gst::State::Null)?;
        info!("Video processing completed");

        Ok(())
    }
}
