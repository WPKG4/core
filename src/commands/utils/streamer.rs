use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;

use anyhow::{Context, anyhow};
use gstreamer::prelude::*;
use gstreamer_app::AppSrc;
use scap::capturer::{Capturer, Options};

#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub width: i32,
    pub height: i32,
    pub scale_w: i32,
    pub scale_h: i32,
    pub fps: u32,
    pub bitrate: u32,
    pub rtsp_url: String,
    pub show_cursor: bool,
    pub max_buffer_size: usize,
    pub capture_fps: u32,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            scale_w: 1920,
            scale_h: 1080,
            fps: 24,
            bitrate: 5_500_000,
            rtsp_url: "rtsp://localhost:8554/stream".into(),
            show_cursor: true,
            max_buffer_size: 5,
            capture_fps: 30,
        }
    }
}

impl StreamConfig {
    pub fn from_args(args: &HashMap<String, String>) -> anyhow::Result<Self> {
        let rtsp_url =
            args.get("rtsp_url").ok_or_else(|| anyhow!("Missing required parameter 'rtsp_url'"))?;

        let mut config = Self { rtsp_url: rtsp_url.clone(), ..Default::default() };

        if let Some(width) = args.get("width") {
            config.width = width.parse().with_context(|| format!("Invalid width: {}", width))?;
        }

        if let Some(height) = args.get("height") {
            config.height =
                height.parse().with_context(|| format!("Invalid height: {}", height))?;
        }

        if let Some(scale_w) = args.get("scale_w") {
            config.scale_w =
                scale_w.parse().with_context(|| format!("Invalid scale_w: {}", scale_w))?;
        }

        if let Some(scale_h) = args.get("scale_h") {
            config.scale_h =
                scale_h.parse().with_context(|| format!("Invalid scale_h: {}", scale_h))?;
        }

        if let Some(fps) = args.get("fps") {
            config.fps = fps.parse().with_context(|| format!("Invalid fps: {}", fps))?;
        }

        if let Some(bitrate) = args.get("bitrate") {
            config.bitrate =
                bitrate.parse().with_context(|| format!("Invalid bitrate: {}", bitrate))?;
        }

        if let Some(show_cursor) = args.get("show_cursor") {
            config.show_cursor = show_cursor
                .parse()
                .with_context(|| format!("Invalid show_cursor: {}", show_cursor))?;
        }

        if let Some(max_buffer_size) = args.get("max_buffer_size") {
            config.max_buffer_size = max_buffer_size
                .parse()
                .with_context(|| format!("Invalid max_buffer_size: {}", max_buffer_size))?;
        }

        if let Some(capture_fps) = args.get("capture_fps") {
            config.capture_fps = capture_fps
                .parse()
                .with_context(|| format!("Invalid capture_fps: {}", capture_fps))?;
        }

        Ok(config)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Stopped,
    Running,
    Error,
}

pub struct ScreenStreamer {
    config: StreamConfig,
    state: Arc<Mutex<StreamState>>,
    eos_sent: Arc<AtomicBool>,
    pipeline: Arc<Mutex<Option<gstreamer::Pipeline>>>,
    frame_buffer: Arc<(Mutex<VecDeque<Vec<u8>>>, Condvar)>,
    capture_thread: Mutex<Option<JoinHandle<()>>>,
    capture_stop: Arc<AtomicBool>,
}

impl ScreenStreamer {
    pub fn new() -> Self {
        Self {
            config: StreamConfig::default(),
            state: Arc::new(Mutex::new(StreamState::Stopped)),
            eos_sent: Arc::new(AtomicBool::new(false)),
            pipeline: Arc::new(Mutex::new(None)),
            frame_buffer: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
            capture_thread: Mutex::new(None),
            capture_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn configure(&mut self, config: StreamConfig) {
        self.config = config;
    }

    pub fn start(&self) -> anyhow::Result<()> {
        let mut state = self.state.lock().map_err(|e| anyhow!("Failed to lock state: {}", e))?;
        if *state != StreamState::Stopped {
            return Err(anyhow!("Stream already running"));
        }

        *state = StreamState::Running;
        self.eos_sent.store(false, Ordering::SeqCst);
        self.capture_stop.store(false, Ordering::SeqCst);

        self.start_capturer()?;

        let pipeline = self.create_pipeline()?;
        let pipeline_clone = pipeline.clone();

        self.pipeline
            .lock()
            .map_err(|e| anyhow!("Failed to lock pipeline: {}", e))?
            .replace(pipeline);

        let state_clone = Arc::clone(&self.state);

        thread::spawn(move || {
            if let Err(e) = Self::main_loop(pipeline_clone, state_clone) {
                eprintln!("Stream error: {}", e);
            }
        });

        Ok(())
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.eos_sent.store(true, Ordering::SeqCst);
        self.capture_stop.store(true, Ordering::SeqCst);

        let (_, cvar) = &*self.frame_buffer;
        cvar.notify_all();

        if let Ok(mut handle_guard) = self.capture_thread.lock() {
            if let Some(handle) = handle_guard.take() {
                if handle.join().is_err() {
                    return Err(anyhow::anyhow!("Failed to stop stream!"));
                }
            }
        }

        if let Ok(pipeline_guard) = self.pipeline.lock() {
            if let Some(pipeline) = pipeline_guard.as_ref() {
                let _ = pipeline.send_event(gstreamer::event::Eos::new());
            }
        }
        Ok(())
    }

    pub fn state(&self) -> StreamState {
        self.state.lock().map(|guard| *guard).unwrap_or_else(|e| {
            eprintln!("Failed to get state: {}", e);
            StreamState::Error
        })
    }

    fn start_capturer(&self) -> anyhow::Result<()> {
        let config = self.config.clone();
        let frame_buffer = Arc::clone(&self.frame_buffer);
        let stop_flag = Arc::clone(&self.capture_stop);

        let handle = thread::spawn(move || {
            let mut capturer = match Capturer::build(Options {
                fps: config.capture_fps,
                show_cursor: config.show_cursor,
                output_type: scap::frame::FrameType::BGRAFrame,
                ..Default::default()
            }) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to create capturer: {}", e);
                    return;
                }
            };

            capturer.start_capture();

            println!("[CAPTURE] Capturer started");

            let expected_size = (config.width * config.height * 4) as usize;

            while !stop_flag.load(Ordering::Relaxed) {
                match capturer.get_next_frame() {
                    Ok(frame) => {
                        if let scap::frame::Frame::BGRx(bgrx_frame) = frame {
                            let (lock, cvar) = &*frame_buffer;
                            let mut buf = match lock.lock() {
                                Ok(guard) => guard,
                                Err(e) => {
                                    eprintln!("[CAPTURE] Failed to lock buffer: {}", e);
                                    continue;
                                }
                            };

                            if bgrx_frame.data.len() != expected_size {
                                eprintln!(
                                    "[CAPTURE] Invalid frame size: {} (expected {})",
                                    bgrx_frame.data.len(),
                                    expected_size
                                );
                                continue;
                            }

                            if buf.len() >= config.max_buffer_size {
                                buf.pop_front();
                            }

                            buf.push_back(bgrx_frame.data);
                            cvar.notify_one();
                        }
                    }
                    Err(e) => eprintln!("[CAPTURE] Error: {}", e),
                }
            }
            println!("[CAPTURE] Capturer stopped");
        });

        self.capture_thread
            .lock()
            .map_err(|e| anyhow!("Failed to lock capture thread: {}", e))?
            .replace(handle);
        Ok(())
    }

    fn create_pipeline(&self) -> anyhow::Result<gstreamer::Pipeline> {
        gstreamer::init().context("Failed to initialize gstreamer")?;

        let pipeline = gstreamer::Pipeline::default();
        let video_info = gstreamer_video::VideoInfo::builder(
            gstreamer_video::VideoFormat::Bgrx,
            self.config.width as u32,
            self.config.height as u32,
        )
        .fps(gstreamer::Fraction::new(self.config.fps as i32, 1))
        .build()
        .map_err(|e| anyhow!("Video info error: {:?}", e))?;

        let appsrc = AppSrc::builder()
            .caps(&video_info.to_caps().map_err(|e| anyhow!("Caps error: {:?}", e))?)
            .format(gstreamer::Format::Time)
            .build();

        appsrc.set_property("is-live", true);
        appsrc.set_property("do-timestamp", true);
        appsrc.set_property_from_str("stream-type", "stream");
        appsrc.set_property("block", true);

        let elements = [
            appsrc.upcast_ref(),
            &gstreamer::ElementFactory::make("videoconvert")
                .property("n-threads", 2u32)
                .build()
                .context("Failed to create videoconvert")?,
            &gstreamer::ElementFactory::make("videoscale")
                .property_from_str("method", "1")
                .build()
                .context("Failed to create videoscale")?,
            &gstreamer::ElementFactory::make("capsfilter")
                .property(
                    "caps",
                    gstreamer_video::VideoInfo::builder(
                        gstreamer_video::VideoFormat::I420,
                        self.config.scale_w as u32,
                        self.config.scale_h as u32,
                    )
                    .fps(gstreamer::Fraction::new(self.config.fps as i32, 1))
                    .build()
                    .map_err(|e| anyhow!("Scale caps error: {:?}", e))?
                    .to_caps()
                    .map_err(|e| anyhow!("Scale caps conversion error: {:?}", e))?,
                )
                .build()
                .context("Failed to create capsfilter")?,
            &gstreamer::ElementFactory::make("openh264enc")
                .property("bitrate", self.config.bitrate)
                .property_from_str("rate-control", "bitrate")
                .property("multi-thread", 2u32)
                .property("gop-size", 48u32)
                .property_from_str("slice-mode", "auto")
                .property("max-bitrate", (self.config.bitrate as f32 * 1.4) as u32)
                .property_from_str("complexity", "low")
                .property("qp-min", 20u32)
                .property("qp-max", 40u32)
                .build()
                .context("Failed to create openh264enc")?,
            &gstreamer::ElementFactory::make("h264parse")
                .build()
                .context("Failed to create h264parse")?,
            &gstreamer::ElementFactory::make("rtspclientsink")
                .property("location", &self.config.rtsp_url)
                .property("async-handling", true)
                .property_from_str("protocols", "tcp")
                .build()
                .context("Failed to create rtspclientsink")?,
        ];

        pipeline.add_many(elements).context("Failed to add elements to pipeline")?;
        gstreamer::Element::link_many(elements).context("Failed to link elements")?;

        let buffer_ref = Arc::clone(&self.frame_buffer);
        let eos_sent_clone = Arc::clone(&self.eos_sent);
        let config = self.config.clone();

        appsrc.set_callbacks(
            gstreamer_app::AppSrcCallbacks::builder()
                .need_data(move |appsrc, _| {
                    if eos_sent_clone.load(Ordering::SeqCst) {
                        return;
                    }

                    let (lock, cvar) = &*buffer_ref;
                    let mutex_guard = match lock.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            eprintln!("[APPSRC] Failed to lock buffer: {}", e);
                            return;
                        }
                    };

                    let mut guard = match cvar.wait_while(mutex_guard, |g| {
                        g.is_empty() && !eos_sent_clone.load(Ordering::SeqCst)
                    }) {
                        Ok(g) => g,
                        Err(_) => return,
                    };

                    if let Some(data) = guard.pop_front() {
                        let expected_size = (config.width * config.height * 4) as usize;
                        if data.len() != expected_size {
                            eprintln!("[APPSRC] Invalid frame size");
                            return;
                        }

                        let mut buffer = match gstreamer::Buffer::with_size(expected_size) {
                            Ok(b) => b,
                            Err(e) => {
                                eprintln!("[APPSRC] Buffer error: {}", e);
                                return;
                            }
                        };

                        {
                            let buffer = buffer.make_mut();
                            match gstreamer_video::VideoFrameRef::from_buffer_ref_writable(
                                buffer,
                                &video_info,
                            ) {
                                Ok(mut vframe) => {
                                    if let Ok(dest) = vframe.plane_data_mut(0) {
                                        dest.copy_from_slice(&data);
                                    } else {
                                        eprintln!("[APPSRC] Failed to get plane data");
                                        return;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[APPSRC] Failed to create video frame: {}", e);
                                    return;
                                }
                            }
                        }

                        if let Err(e) = appsrc.push_buffer(buffer) {
                            eprintln!("[APPSRC] Push error: {}", e);
                        }
                    }
                })
                .build(),
        );

        Ok(pipeline)
    }

    fn main_loop(
        pipeline: gstreamer::Pipeline,
        state: Arc<Mutex<StreamState>>,
    ) -> anyhow::Result<()> {
        pipeline
            .set_state(gstreamer::State::Playing)
            .context("Failed to set pipeline to playing")?;
        println!("[PIPELINE] Started playing");

        let bus = pipeline.bus().ok_or_else(|| anyhow!("Pipeline has no bus"))?;
        for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
            use gstreamer::MessageView;
            match msg.view() {
                MessageView::Eos(..) => {
                    println!("[PIPELINE] EOS received");
                    break;
                }
                MessageView::Error(err) => {
                    pipeline
                        .set_state(gstreamer::State::Null)
                        .context("Failed to set pipeline to null")?;
                    *state.lock().map_err(|e| anyhow!("Failed to lock state: {}", e))? =
                        StreamState::Error;
                    return Err(anyhow!(
                        "[ERROR] {}, {}",
                        err.error(),
                        err.details()
                            .map(|s| s.to_string())
                            .unwrap_or("No error details!".to_string())
                    ));
                }
                MessageView::StateChanged(state_change) => {
                    if state_change.current() == gstreamer::State::Playing {
                        println!("[PIPELINE] Successfully playing");
                    }
                }
                _ => (),
            }
        }

        pipeline.set_state(gstreamer::State::Null).context("Failed to set pipeline to null")?;
        *state.lock().map_err(|e| anyhow!("Failed to lock state: {}", e))? = StreamState::Stopped;
        println!("[PIPELINE] Stopped");
        Ok(())
    }
}

impl Default for ScreenStreamer {
    fn default() -> Self {
        Self::new()
    }
}
