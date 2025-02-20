use anyhow::Result;
use gstreamer as gst;
use gstreamer_video as gst_video;
use gstreamer_app as gst_app;
use std::path::Path;
use log::{info, error, debug};

use gst::prelude::*;

pub use gst_video::video_frame::{VideoFrame, Readable, Writable, VideoFrameExt};

pub struct VideoProcessor {
    pipeline: gst::Pipeline,
}

impl VideoProcessor {
    pub fn new() -> Result<Self> {
        info!("Initializing GStreamer");
        gst::init()?;
        let pipeline = gst::Pipeline::new();
        info!("GStreamer pipeline created");
        Ok(Self { pipeline })
    }

    pub fn process_video<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
        frame_callback: impl Fn(&mut VideoFrame<Writable>) -> Result<()> + Send + 'static,
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
            .property("max-size-buffers", 4u32)
            .build()?;
        let appsink = gst::ElementFactory::make("appsink").build()?;
        
        let queue2 = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 4u32)
            .build()?;
        let appsrc = gst::ElementFactory::make("appsrc").build()?;
        let videoconvert2 = gst::ElementFactory::make("videoconvert").build()?;
        let videoscale2 = gst::ElementFactory::make("videoscale").build()?;
        // maybe adding nvdec or vaapih264dec here would be better for performance? cfg!(feature = "hardware-acceleration") smth like that
        // Create encoder with optimal settings
        let x264enc = gst::ElementFactory::make("x264enc")
            .property_from_str("speed-preset", "ultrafast")  //ultrafast for better performance
            .property("bitrate", 2048u32)
            .property("key-int-max", 25u32)
            .property_from_str("tune", "zerolatency")
            .build()?;

        let h264parse = gst::ElementFactory::make("h264parse").build()?;
        let queue3 = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 4u32)
            .build()?;
        let muxer = gst::ElementFactory::make("mp4mux").build()?;
        let sink = gst::ElementFactory::make("filesink")
            .property("location", output_path.to_str().unwrap())
            .property("sync", false)
            .property("async", false)
            .build()?;

        // Create audio processing elements
        let queue_audio = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 4u32)
            .build()?;
        let audioconvert = gst::ElementFactory::make("audioconvert").build()?;
        let audioresample = gst::ElementFactory::make("audioresample").build()?;
        let aacenc = gst::ElementFactory::make("avenc_aac").build()?;
        let aacparse = gst::ElementFactory::make("aacparse").build()?;

        let appsrc = appsrc.dynamic_cast::<gst_app::AppSrc>().unwrap();
        appsrc.set_format(gst::Format::Time);
        appsrc.set_property("is-live", false);
        appsrc.set_property("do-timestamp", true);
        appsrc.set_property("block", true);
        appsrc.set_property("format", gst::Format::Time);

        let appsink = appsink.dynamic_cast::<gst_app::AppSink>().unwrap();
        appsink.set_property("sync", false);
        appsink.set_property("drop", true);
        appsink.set_property("max-buffers", 1u32);

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
            &x264enc,
            &h264parse,
            &queue3,
            &queue_audio,
            &audioconvert,
            &audioresample,
            &aacenc,
            &aacparse,
            &muxer,
            &sink,
        ])?;

        gst::Element::link_many(&[&videoconvert1, &videoscale1, &queue1, &appsink.upcast_ref()])?;
        gst::Element::link_many(&[
            &appsrc.upcast_ref(),
            &queue2,
            &videoconvert2,
            &videoscale2,
            &x264enc,
            &h264parse,
            &queue3,
            &muxer,
            &sink,
        ])?;

        gst::Element::link_many(&[
            &queue_audio,
            &audioconvert,
            &audioresample,
            &aacenc,
            &aacparse,
            &muxer,
        ])?;

        gst::Element::link_many(&[&src, &decodebin])?;

        // Set caps for appsink
        appsink.set_caps(Some(&gst::Caps::builder("video/x-raw")
            .field("format", gst_video::VideoFormat::Rgba.to_str())
            .build()));

        let mut video_info: Option<gst_video::VideoInfo> = None;
        let appsrc_weak = appsrc.downgrade();
        let frame_count = std::sync::atomic::AtomicUsize::new(0);
        
        // Clone for new_sample closure
        let appsrc_weak_cb = appsrc_weak.clone();
        // Clone for eos closure
        let appsrc_weak_eos = appsrc_weak.clone();

        // Setup callbacks
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let count = frame_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    debug!("Processing frame {}", count);

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

                    // Set up appsrc caps if this is the first frame
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

                    // Push processed frame to appsrc
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
                })
                .build(),
        );

        let videoconvert1_weak = videoconvert1.downgrade();
        let queue_audio_weak = queue_audio.downgrade();
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
                if let Some(queue) = queue_audio_weak.upgrade() {
                    let sink_pad = queue.static_pad("sink").unwrap();
                    if let Err(e) = pad.link(&sink_pad) {
                        error!("Failed to link decoder to audio queue: {}", e);
                    } else {
                        info!("Linked decoder to audio queue successfully");
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
        let appsrc_weak_eos = appsrc_weak.clone();

        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            match msg.view() {
                gst::MessageView::Eos(..) => {
                    info!("End of stream reached");
                    if let Some(appsrc) = appsrc_weak_eos.upgrade() {
                        appsrc.end_of_stream()?;
                    }
                    break;
                },
                gst::MessageView::Error(err) => {
                    error!("Pipeline error: {} ({:?})", err.error(), err.debug());
                    // Set pipeline to NULL state before returning error
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

        // Clean up -  set to NULL state
        info!("Cleaning up pipeline");
        self.pipeline.set_state(gst::State::Null)?;
        info!("Video processing completed");

        Ok(())
    }
}