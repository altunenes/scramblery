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

        debug!("Creating pipeline elements");
        let src = gst::ElementFactory::make("filesrc")
            .property("location", input_path.to_str().unwrap())
            .build()?;
        info!("Created filesrc element");

        let decodebin = gst::ElementFactory::make("decodebin").build()?;
        info!("Created decodebin element");

        let videoconvert = gst::ElementFactory::make("videoconvert").build()?;
        info!("Created videoconvert element");

        let appsink = gst::ElementFactory::make("appsink").build()?;
        info!("Created appsink element");

        debug!("Adding elements to pipeline");
        self.pipeline.add_many(&[&src, &decodebin, &videoconvert, &appsink])?;
        gst::Element::link_many(&[&src, &decodebin])?;
        gst::Element::link_many(&[&videoconvert, &appsink])?;
        info!("Pipeline elements linked");

        let appsink = appsink.dynamic_cast::<gst_app::AppSink>().unwrap();
        appsink.set_caps(Some(&gst::Caps::builder("video/x-raw")
            .field("format", gst_video::VideoFormat::Rgba.to_str())
            .build()));
        info!("AppSink caps set");

        let frame_count = std::sync::atomic::AtomicUsize::new(0);
        
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let count = frame_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    debug!("Processing frame {}", count);

                    let sample = appsink.pull_sample().map_err(|_| {
                        error!("Failed to pull sample");
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

                    let video_info = gst_video::VideoInfo::from_caps(caps).map_err(|_| {
                        error!("Failed to get video info from caps");
                        gst::FlowError::Error
                    })?;

                    let mut buffer = buffer.to_owned();
                    let mut frame = gst_video::VideoFrame::from_buffer_writable(buffer, &video_info)
                        .map_err(|_| {
                            error!("Failed to create writable frame");
                            gst::FlowError::Error
                        })?;

                    if let Err(e) = frame_callback(&mut frame) {
                        error!("Frame callback error: {}", e);
                        return Err(gst::FlowError::Error);
                    }

                    debug!("Frame {} processed successfully", count);
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        info!("Setting up decodebin pad-added handler");
        decodebin.connect_pad_added(move |_, pad| {
            let caps = pad.current_caps().unwrap();
            let structure = caps.structure(0).unwrap();
            debug!("New pad added with caps: {:?}", structure.name());

            if structure.name().starts_with("video/") {
                let sink_pad = videoconvert.static_pad("sink").unwrap();
                if let Err(e) = pad.link(&sink_pad) {
                    error!("Failed to link decoder to converter: {}", e);
                } else {
                    info!("Linked decoder to converter successfully");
                }
            }
        });

        info!("Starting pipeline");
        self.pipeline.set_state(gst::State::Playing)?;

        info!("Waiting for pipeline to finish");
        let bus = self.pipeline.bus().unwrap();
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            match msg.view() {
                gst::MessageView::Eos(..) => {
                    info!("End of stream reached");
                    break;
                },
                gst::MessageView::Error(err) => {
                    error!("Pipeline error: {} ({:?})", err.error(), err.debug());
                    return Err(anyhow::anyhow!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    ));
                }
                gst::MessageView::StateChanged(state) => {
                    debug!("Pipeline state changed: {:?}", state);
                }
                _ => (),
            }
        }

        info!("Cleaning up pipeline");
        self.pipeline.set_state(gst::State::Null)?;
        info!("Video processing completed");

        Ok(())
    }
}