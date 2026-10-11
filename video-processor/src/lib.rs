use anyhow::Result;
use gstreamer as gst;
use gstreamer_video as gst_video;
use gstreamer_app as gst_app;
use std::path::Path;
use log::{info, warn, error, debug};
use gstreamer_pbutils::Discoverer;
use gst_video::prelude::*;
use gstreamer_pbutils::prelude::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum VideoFidelity {
    /// Lossless 4:4:4 (H.264, or 10-bit HEVC for sources above 8 bits) with software decoding:
    /// decoded frames match the processed ones up to RGB to YUV rounding.
    Lossless,
    /// 4:4:4 with mild quantisation and software decoding.
    HighQuality,
    /// Hardware decoding and encoding where available, 4:2:0.
    #[default]
    Fast,
}

/// How frames are grouped for processing.
#[derive(Debug, Clone, Copy)]
pub struct Batching {
    /// Largest number of frames per batch.
    pub max_frames: usize,
    /// Approximate peak working memory per pixel of each frame in a batch.
    pub bytes_per_pixel: usize,
    /// Memory allowed for one batch; batches are shortened to stay within it.
    pub memory_budget: usize,
    /// When batches are limited by memory, make them equally long (using the frame count
    /// estimate) instead of leaving a short final batch.
    pub balance: bool,
    /// Number of frames following each batch that are passed along with it as lookahead.
    pub overlap: usize,
}

/// The YUV to RGB matrix a video declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMatrix {
    Bt601,
    Bt709,
    Bt2020,
}

/// A decoded frame as full-resolution (4:4:4) Y, Cb and Cr planes, row-major without padding,
/// at the source's bit depth (`max_value` = 2^bits - 1). Limited-range levels at N bits are the
/// 8-bit levels multiplied by 2^(N - 8).
#[derive(Debug, Clone)]
pub struct YuvFrame {
    /// Position of the frame in the video, starting at 0.
    pub index: usize,
    pub width: usize,
    pub height: usize,
    pub y: Vec<u16>,
    pub u: Vec<u16>,
    pub v: Vec<u16>,
    pub max_value: u16,
    pub matrix: ColorMatrix,
    /// Full range rather than limited range (16..235 for Y, 16..240 for chroma at 8 bits).
    pub full_range: bool,
    /// The frame is passed as context for the next batch (see `Batching::overlap`) and is not
    /// written in this batch; it is passed again, as a regular frame, with the next batch.
    pub lookahead: bool,
}

fn sample_bits(info: &gst_video::VideoInfo) -> u32 {
    info.format_info().depth()[0]
}

/// The 4:4:4 format that holds samples of `bits` bits without conversion.
fn native_444_format(bits: u32) -> &'static str {
    match bits {
        0..=8 => "Y444",
        9..=10 => "Y444_10LE",
        11..=12 => "Y444_12LE",
        _ => "Y444_16LE",
    }
}

/// The colorimetry string for `matrix` (with its usual transfer function and primaries),
/// keeping the value range of `caps`.
fn colorimetry_with_matrix(caps: &gst::CapsRef, matrix: ColorMatrix) -> Option<String> {
    use gst_video::{VideoColorMatrix as M, VideoColorPrimaries as P, VideoTransferFunction as T};
    let info = gst_video::VideoInfo::from_caps(caps).ok()?;
    let (m, t, p) = match matrix {
        ColorMatrix::Bt601 => (M::Bt601, T::Bt709, P::Smpte170m),
        ColorMatrix::Bt709 => (M::Bt709, T::Bt709, P::Bt709),
        ColorMatrix::Bt2020 => (M::Bt2020, T::Bt202012, P::Bt2020),
    };
    Some(gst_video::VideoColorimetry::new(info.colorimetry().range(), m, t, p).to_string())
}

/// `caps` with the colorimetry for `matrix`.
fn caps_with_matrix(caps: &gst::CapsRef, matrix: ColorMatrix) -> gst::Caps {
    let mut out = caps.to_owned();
    if let Some(colorimetry) = colorimetry_with_matrix(caps, matrix) {
        if let Some(s) = out.make_mut().structure_mut(0) {
            s.set("colorimetry", colorimetry.as_str());
        }
    }
    out
}

fn color_matrix(info: &gst_video::VideoInfo) -> ColorMatrix {
    match info.colorimetry().matrix() {
        gst_video::VideoColorMatrix::Bt601 | gst_video::VideoColorMatrix::Fcc => ColorMatrix::Bt601,
        gst_video::VideoColorMatrix::Bt2020 => ColorMatrix::Bt2020,
        _ => ColorMatrix::Bt709,
    }
}

fn read_frame(buffer: &gst::Buffer, info: &gst_video::VideoInfo, index: usize) -> Result<YuvFrame> {
    let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer.as_ref(), info)
        .map_err(|_| anyhow::anyhow!("Cannot map video frame"))?;
    let (w, h) = (info.width() as usize, info.height() as usize);
    let bits = sample_bits(info);
    let wide = bits > 8;
    let plane = |i: u32| -> Result<Vec<u16>> {
        let data = frame.plane_data(i).map_err(|_| anyhow::anyhow!("Missing plane {i}"))?;
        let stride = frame.plane_stride()[i as usize] as usize;
        let mut out = Vec::with_capacity(w * h);
        for row in 0..h {
            let line = &data[row * stride..];
            if wide {
                out.extend(line[..2 * w].as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes(*b)));
            } else {
                out.extend(line[..w].iter().map(|&b| b as u16));
            }
        }
        Ok(out)
    };
    Ok(YuvFrame {
        index,
        width: w,
        height: h,
        y: plane(0)?,
        u: plane(1)?,
        v: plane(2)?,
        max_value: ((1u32 << bits) - 1) as u16,
        lookahead: false,
        matrix: color_matrix(info),
        full_range: info.colorimetry().range() == gst_video::VideoColorRange::Range0_255,
    })
}

/// A new buffer holding `yuv`, with the timestamps of the decoded `input` buffer.
fn write_frame(input: &gst::Buffer, info: &gst_video::VideoInfo, yuv: &YuvFrame) -> Result<gst::Buffer> {
    let mut buffer = gst::Buffer::with_size(info.size())?;
    {
        let out = buffer.get_mut().ok_or_else(|| anyhow::anyhow!("New buffer is not writable"))?;
        out.set_pts(input.pts());
        out.set_dts(input.dts());
        out.set_duration(input.duration());
    }
    let mut frame = gst_video::VideoFrame::from_buffer_writable(buffer, info)
        .map_err(|_| anyhow::anyhow!("Cannot map video frame for writing"))?;
    let w = yuv.width;
    let wide = sample_bits(info) > 8;
    for (i, src) in [&yuv.y, &yuv.u, &yuv.v].into_iter().enumerate() {
        let stride = frame.plane_stride()[i] as usize;
        let data = frame.plane_data_mut(i as u32).map_err(|_| anyhow::anyhow!("Missing plane {i}"))?;
        for row in 0..yuv.height {
            let src = &src[row * w..(row + 1) * w];
            let line = &mut data[row * stride..];
            if wide {
                for (dst, v) in line[..2 * w].as_chunks_mut::<2>().0.iter_mut().zip(src) {
                    *dst = v.to_le_bytes();
                }
            } else {
                for (dst, &v) in line[..w].iter_mut().zip(src) {
                    *dst = v as u8;
                }
            }
        }
    }
    Ok(frame.into_buffer())
}

/// Bit depth of the decoded video (8 when it cannot be determined).
fn decoded_depth(input: &Path) -> u32 {
    let location = input.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    let Ok(element) = gst::parse::launch(&format!("filesrc location=\"{location}\" ! decodebin ! fakesink name=sink")) else {
        return 8;
    };
    let Ok(pipeline) = element.dynamic_cast::<gst::Pipeline>() else { return 8 };
    let _ = pipeline.set_state(gst::State::Paused);
    let _ = pipeline.state(gst::ClockTime::from_seconds(5));
    let depth = pipeline
        .by_name("sink")
        .and_then(|sink| sink.static_pad("sink"))
        .and_then(|pad| pad.current_caps())
        .and_then(|caps| gst_video::VideoInfo::from_caps(&caps).ok())
        .map(|info| info.format_info().depth()[0])
        .unwrap_or(8);
    let _ = pipeline.set_state(gst::State::Null);
    depth
}

/// An encoder for 10-bit 4:4:4 HEVC (x265), used for sources with more than 8 bits.
fn make_high_depth_encoder(fidelity: VideoFidelity) -> Option<(gst::Element, &'static str)> {
    let builder = gst::ElementFactory::make("x265enc")
        .property_from_str("speed-preset", "superfast")
        .property("key-int-max", 25i32);
    let enc = match fidelity {
        VideoFidelity::Lossless => builder.property("option-string", "lossless=1").build(),
        VideoFidelity::HighQuality => builder.property("qp", 18i32).build(),
        VideoFidelity::Fast => return None,
    }
    .ok()?;
    info!("Using x265enc software encoder, 10-bit 4:4:4 ({:?})", fidelity);
    Some((enc, "Y444_10LE"))
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

    let hardware: [(&str, &str, &[(&str, &str)]); 4] = [
        ("vtenc_h264", "VideoToolbox", &[("bitrate", "2048"), ("max-keyframe-interval", "25"), ("realtime", "true"), ("allow-frame-reordering", "false")]),
        ("mfh264enc", "Media Foundation", &[("bitrate", "2048"), ("gop-size", "25")]),
        ("nvh264enc", "NVENC", &[("bitrate", "2048"), ("gop-size", "25"), ("preset", "low-latency-hp"), ("rc-mode", "cbr")]),
        ("vaapih264enc", "VA-API", &[("bitrate", "2048"), ("keyframe-period", "25")]),
    ];
    for (factory, name, properties) in hardware {
        if let Ok(enc) = gst::ElementFactory::make(factory).build() {
            // Property names differ between GStreamer versions, and setting a missing one panics.
            for (property, value) in properties {
                if enc.has_property(property) {
                    enc.set_property_from_str(property, value);
                } else {
                    warn!("{factory} has no property {property}; leaving it at its default");
                }
            }
            info!("Using {name} H.264 encoder (hardware)");
            return Ok((enc, "NV12"));
        }
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

/// Hardware decoders, which are fast but may output 8-bit frames and drop the colour description.
const HARDWARE_DECODERS: &[&str] = &[
    "vtdec_hw", "vtdec",
    "nvh264dec", "nvh264sldec", "nvh265dec", "nvh265sldec", "nvav1dec",
    "vaapih264dec", "vaapih265dec", "vampeg2dec", "vaapiav1dec",
    "d3d11h264dec", "d3d11h265dec", "d3d11av1dec",
    "d3d12h264dec", "d3d12h265dec",
    "vulkanh264dec", "vulkanh265dec", "vulkanav1dec",
];

/// Ranks hardware decoders first (fast) or disables them so that software decoders, which keep
/// the full bit depth and colorimetry, are used.
fn prefer_hardware_decoders(enable: bool) {
    let registry = gst::Registry::get();
    let rank = if enable { gst::Rank::PRIMARY + 2 } else { gst::Rank::NONE };
    for name in HARDWARE_DECODERS {
        if let Some(feature) = registry.lookup_feature(name) {
            feature.set_rank(rank);
        }
    }
    info!("{} hardware decoders", if enable { "Preferring" } else { "Disabled" });
}

impl VideoProcessor {
    pub fn new() -> Result<Self> {
        info!("Initializing GStreamer");
        gst::init()?;

        let pipeline = gst::Pipeline::new();
        info!("GStreamer pipeline created");
        Ok(Self { pipeline })
    }

    /// Processes a video and returns the bit depth of the output.
    #[allow(clippy::too_many_arguments)]
    pub fn process_video<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
        fidelity: VideoFidelity,
        batching: Batching,
        matrix_override: Option<ColorMatrix>,
        frame_callback: impl Fn(&mut [YuvFrame]) -> Result<()> + Send + Sync + 'static,
        progress_callback: impl Fn(f32) + Send + Sync + 'static,
    ) -> Result<u32> {
        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();
        // Exact output needs exact decoding; only the Fast setting trades it for speed.
        prefer_hardware_decoders(fidelity == VideoFidelity::Fast);
        info!("Setting up video processing pipeline");
        info!("Input path: {:?}", input_path);
        info!("Output path: {:?}", output_path);

        let src = gst::ElementFactory::make("filesrc")
            .property("location", input_path.to_str().unwrap())
            .build()?;

        let decodebin = gst::ElementFactory::make("decodebin").build()?;
        let videoconvert1 = gst::ElementFactory::make("videoconvert").property_from_str("dither", "none").build()?;
        let videoscale1 = gst::ElementFactory::make("videoscale").build()?;
        let queue1 = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 2u32)
            .build()?;
        let appsink = gst::ElementFactory::make("appsink").build()?;

        // Processed raw frames waiting for the encoder. Bounded so that a slow encoder makes
        // processing wait instead of piling up uncompressed frames in memory.
        let queue2 = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 4u32)
            .property("max-size-time", 0u64)
            .property("max-size-bytes", 0u32)
            .build()?;
        let appsrc = gst::ElementFactory::make("appsrc").build()?;
        // Any change of bit depth towards the encoder rounds instead of dithering.
        let videoconvert2 = gst::ElementFactory::make("videoconvert").property_from_str("dither", "none").build()?;
        let videoscale2 = gst::ElementFactory::make("videoscale").build()?;
        let input_depth = decoded_depth(input_path);
        let (encoder, caps_format, output_depth, parser) = match (input_depth > 8).then(|| make_high_depth_encoder(fidelity)).flatten() {
            Some((enc, format)) => (enc, format, 10, "h265parse"),
            None => {
                let (enc, format) = make_h264_encoder(fidelity)?;
                (enc, format, 8, "h264parse")
            }
        };
        info!("Input {} bits per sample, output {} bits", input_depth, output_depth);

        let h264parse = gst::ElementFactory::make(parser).build()?;
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
        // Frames keep the timestamps of the decoded input.
        appsrc.set_property("do-timestamp", false);
        appsrc.set_property("block", true);
        appsrc.set_property("format", gst::Format::Time);

        let appsink = appsink.dynamic_cast::<gst_app::AppSink>().unwrap();
        appsink.set_property("sync", false);
        appsink.set_property("drop", false);
        appsink.set_property("max-buffers", 0u32);

        // Audio elements are added in pad-added, only when the input has audio.
        self.pipeline.add_many([
            &src,
            &decodebin,
            &videoconvert1,
            &videoscale1,
            &queue1,
            appsink.upcast_ref(),
            appsrc.upcast_ref(),
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

        gst::Element::link_many([&videoconvert1, &videoscale1, &queue1, appsink.upcast_ref()])?;
        gst::Element::link_many([
            appsrc.upcast_ref(),
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

        gst::Element::link_many([&src, &decodebin])?;

        // Full-resolution YUV at the source's bit depth: no RGB conversion (and its clipping) and
        // no change of bit depth happens inside GStreamer.
        appsink.set_caps(Some(&gst::Caps::builder("video/x-raw")
            .field("format", native_444_format(input_depth))
            .build()));

        let appsrc_weak = appsrc.downgrade();
        let total_frames = {
            let mut frames = 0;
            let timeout = gst::ClockTime::from_seconds(5);
            match Discoverer::new(timeout) {
                Ok(discoverer) => {
                    let path_str = input_path.to_str().unwrap();
                    let uri = if path_str.starts_with("file://") {
                        path_str.to_string()
                    } else {
                        let absolute_path = std::fs::canonicalize(input_path)
                            .unwrap_or(input_path.to_path_buf());
                        // canonicalize gives \\?\C:\... on Windows, which is not a valid file URI path.
                        let path_str = absolute_path.to_string_lossy();
                        let path_str = path_str.trim_start_matches(r"\\?\");
                        gst::glib::filename_to_uri(path_str, None)
                            .map(|uri| uri.to_string())
                            .unwrap_or_else(|_| format!("file:///{}", path_str.replace('\\', "/")))
                    };

                    info!("Trying to discover URI: {}", uri);
                    match discoverer.discover_uri(&uri) {
                        Ok(info) => {
                            if let Some(video_info) = info.video_streams().first() {
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

        // Frames are collected into batches of `batch_size`, handed to the callback together
        // (which may process them in parallel), and pushed downstream in their original order.
        struct Pending {
            info: Option<gst_video::VideoInfo>,
            frames: Vec<(YuvFrame, gst::Buffer)>,
            next_index: usize,
            batch_size: usize,
        }
        let pending = std::sync::Arc::new(std::sync::Mutex::new(Pending {
            info: None,
            frames: Vec::new(),
            next_index: 0,
            batch_size: batching.max_frames.max(1),
        }));
        let frame_callback = std::sync::Arc::new(frame_callback);
        let progress_callback = std::sync::Arc::new(progress_callback);

        let flush = {
            let appsrc_weak = appsrc_weak.clone();
            let frame_callback = frame_callback.clone();
            let progress_callback = progress_callback.clone();
            move |pending: &mut Pending, last: bool| -> std::result::Result<(), gst::FlowError> {
                if pending.frames.is_empty() {
                    return Ok(());
                }
                let info = pending.info.clone().ok_or(gst::FlowError::Error)?;
                let output = if last { pending.frames.len() } else { pending.batch_size.min(pending.frames.len()) };
                // Lookahead frames are kept, unprocessed, for the next batch.
                let carried: Vec<(YuvFrame, gst::Buffer)> = pending.frames[output..].to_vec();
                let (mut frames, buffers): (Vec<YuvFrame>, Vec<gst::Buffer>) = pending.frames.drain(..).unzip();
                for frame in &mut frames[output..] {
                    frame.lookahead = true;
                }
                pending.frames = carried;
                let appsrc = appsrc_weak.upgrade().ok_or(gst::FlowError::Error)?;
                // Errors are posted on the bus so that the pipeline stops instead of waiting for EOS.
                let fail = |e: anyhow::Error| {
                    error!("Frame processing failed: {:#}", e);
                    gst::element_error!(appsrc, gst::LibraryError::Failed, ("Frame processing failed"), ["{:#}", e]);
                    gst::FlowError::Error
                };
                frame_callback(&mut frames).map_err(fail)?;
                for (frame, buffer) in frames.iter().zip(buffers).take(output) {
                    let out = write_frame(&buffer, &info, frame).map_err(fail)?;
                    appsrc.push_buffer(out).map_err(|_| gst::FlowError::Error)?;
                    if total_frames > 0 {
                        progress_callback((((frame.index + 1) as f32 / total_frames as f32) * 100.0).min(99.9));
                    }
                }
                Ok(())
            }
        };
        let flush = std::sync::Arc::new(flush);
        let flush_eos = flush.clone();
        let pending_eos = pending.clone();
        let appsrc_weak_eos = appsrc_weak.clone();
        let appsrc_weak_cb = appsrc_weak.clone();
        let progress_callback_eos = progress_callback.clone();

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| {
                        debug!("No more samples, sending EOS");
                        if let Some(appsrc) = appsrc_weak_cb.upgrade() {
                            let _ = appsrc.end_of_stream();
                        }
                        gst::FlowError::Eos
                    })?;
                    let buffer = sample.buffer_owned().ok_or(gst::FlowError::Error)?;
                    let caps = sample.caps().ok_or(gst::FlowError::Error)?;

                    let mut pending = pending.lock().map_err(|_| gst::FlowError::Error)?;
                    if pending.info.is_none() {
                        // A chosen colour standard replaces what the decoder reported, for the
                        // conversion of frames and for everything downstream.
                        let caps = match matrix_override {
                            Some(matrix) => caps_with_matrix(caps, matrix),
                            None => caps.to_owned(),
                        };
                        let info = gst_video::VideoInfo::from_caps(&caps).map_err(|_| {
                            error!("Failed to get video info from caps");
                            gst::FlowError::Error
                        })?;
                        if let Some(appsrc) = appsrc_weak_cb.upgrade() {
                            appsrc.set_caps(Some(&caps));
                        }
                        // Keep the frames held at once within the memory budget.
                        let frame_bytes = info.width() as usize * info.height() as usize * batching.bytes_per_pixel;
                        let limit = (batching.memory_budget / frame_bytes.max(1)).saturating_sub(batching.overlap).max(1);
                        if limit < pending.batch_size {
                            let size = if batching.balance && total_frames > limit {
                                total_frames.div_ceil(total_frames.div_ceil(limit))
                            } else {
                                limit
                            };
                            info!("Limiting batches to {} frames to stay within memory", size);
                            pending.batch_size = size;
                        }
                        pending.info = Some(info);
                    }
                    let index = pending.next_index;
                    pending.next_index += 1;
                    let info = pending.info.clone().ok_or(gst::FlowError::Error)?;
                    let frame = read_frame(&buffer, &info, index).map_err(|e| {
                        error!("{:#}", e);
                        gst::FlowError::Error
                    })?;
                    pending.frames.push((frame, buffer));
                    if pending.frames.len() >= pending.batch_size + batching.overlap {
                        flush(&mut pending, false)?;
                    }
                    Ok(gst::FlowSuccess::Ok)
                })
                .eos(move |_appsink| {
                    debug!("AppSink received EOS");
                    if let Ok(mut pending) = pending_eos.lock() {
                        if flush_eos(&mut pending, true).is_err() {
                            error!("Failed to process the last frames");
                        }
                    }
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

        // Encode with the input's colorimetry so that colours survive the round trip unchanged,
        // instead of falling back to a size-dependent default (BT.601 for SD frames).
        let capsfilter_weak = capsfilter.downgrade();
        videoconvert1.static_pad("sink").unwrap().add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_, info| {
            if let Some(gst::PadProbeData::Event(event)) = &info.data {
                if let gst::EventView::Caps(c) = event.view() {
                    let colorimetry = match matrix_override {
                        Some(matrix) => colorimetry_with_matrix(c.caps(), matrix),
                        None => c.caps().structure(0).and_then(|s| s.get::<String>("colorimetry").ok()),
                    };
                    if let (Some(colorimetry), Some(filter)) = (colorimetry, capsfilter_weak.upgrade()) {
                        let mut caps: gst::Caps = filter.property("caps");
                        if let Some(s) = caps.make_mut().structure_mut(0) {
                            s.set("colorimetry", colorimetry.as_str());
                        }
                        filter.set_property("caps", &caps);
                        info!("Encoding with input colorimetry {}", colorimetry);
                    }
                }
            }
            gst::PadProbeReturn::Ok
        });

        // decodebin exposes its pads once the streams are known; video and audio are linked here.
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

        info!("Starting pipeline");
        self.pipeline.set_state(gst::State::Playing)?;

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

        info!("Cleaning up pipeline");
        self.pipeline.set_state(gst::State::Null)?;
        info!("Video processing completed");

        Ok(output_depth)
    }
}
