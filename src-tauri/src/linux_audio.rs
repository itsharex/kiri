//! Async libpulse PCM feeds the recording GStreamer pipeline on its clock.
//! Pulse introspection never opens a stream or starts a daemon. Only an explicit
//! recording/check opens the pinned input; screen-sharing consent is unrelated.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use gstreamer::prelude::*;
use libpulse_binding as pulse;

pub const AUDIO_SPEC: crate::record::AudioSpec = crate::record::AudioSpec {
    sample_rate: 48_000,
    channels: 2,
    format: crate::record::AudioSampleFormat::F32,
};
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(3);
pub const AUDIO_UNAVAILABLE: &str = "Linux audio is unavailable. Check the PulseAudio or PipeWire audio service and the selected sound devices.";
pub const MICROPHONE_UNAVAILABLE: &str =
    "No microphone input is available. Select an unmuted input device in your sound settings.";
pub const AUDIO_RECORDING_FAILED: &str = "Audio recording stopped. Check the sound devices and audio service, then start a new recording.";

#[derive(Debug, Clone)]
pub struct AudioSource {
    pub name: String,
    pub description: String,
    index: u32,
    server: String,
    monitor_of: Option<String>,
    muted: bool,
}

/// Static capability only. Do not access devices while rendering an overlay.
pub fn supported() -> bool {
    gstreamer::init().is_ok()
        && [
            "appsrc",
            "audiorate",
            "audioconvert",
            "audioresample",
            "audiomixer",
            "avenc_aac",
            "aacparse",
        ]
        .iter()
        .all(|name| gstreamer::ElementFactory::find(name).is_some())
}

fn local_server_address() -> Result<String> {
    let server = match std::env::var("PULSE_SERVER") {
        Ok(server) if !server.is_empty() => server,
        _ => {
            let runtime = std::env::var_os("XDG_RUNTIME_DIR").context(AUDIO_UNAVAILABLE)?;
            format!(
                "unix:{}",
                std::path::Path::new(&runtime)
                    .join("pulse/native")
                    .to_str()
                    .context(AUDIO_UNAVAILABLE)?
            )
        }
    };
    validate_local_server(&server)?;
    Ok(server)
}

fn validate_local_server(server: &str) -> Result<()> {
    let path = server.strip_prefix("unix:").unwrap_or(server);
    if !std::path::Path::new(path).is_absolute() || path.chars().any(char::is_whitespace) {
        bail!(AUDIO_UNAVAILABLE);
    }
    Ok(())
}

struct PulseConnection {
    context: pulse::context::Context,
    mainloop: pulse::mainloop::standard::Mainloop,
    deadline: Instant,
}

impl PulseConnection {
    fn new() -> Result<Self> {
        Self::new_at(None)
    }

    fn new_at(server: Option<&str>) -> Result<Self> {
        let mainloop =
            pulse::mainloop::standard::Mainloop::new().ok_or_else(|| anyhow!(AUDIO_UNAVAILABLE))?;
        let server = server
            .map(str::to_owned)
            .map(Ok)
            .unwrap_or_else(local_server_address)?;
        validate_local_server(&server)?;
        let mut context = pulse::context::Context::new(&mainloop, "Kiri audio device check")
            .ok_or_else(|| anyhow!(AUDIO_UNAVAILABLE))?;
        context
            .connect(Some(&server), pulse::context::FlagSet::NOAUTOSPAWN, None)
            .map_err(|_| anyhow!(AUDIO_UNAVAILABLE))?;
        let mut connection = Self {
            context,
            mainloop,
            deadline: Instant::now() + DISCOVERY_TIMEOUT,
        };
        while connection.context.get_state() != pulse::context::State::Ready {
            connection.iterate()?;
        }
        if connection.context.is_local() != Some(true) {
            bail!("Kiri audio recording requires a local sound service.");
        }
        Ok(connection)
    }

    fn iterate(&mut self) -> Result<()> {
        if Instant::now() >= self.deadline
            || matches!(
                self.context.get_state(),
                pulse::context::State::Failed | pulse::context::State::Terminated
            )
        {
            bail!(AUDIO_UNAVAILABLE);
        }
        if !matches!(
            self.mainloop.iterate(false),
            pulse::mainloop::standard::IterateResult::Success(_)
        ) {
            bail!(AUDIO_UNAVAILABLE);
        }
        std::thread::sleep(Duration::from_millis(2));
        Ok(())
    }

    fn complete<T: ?Sized>(&mut self, mut operation: pulse::operation::Operation<T>) -> Result<()> {
        while operation.get_state() == pulse::operation::State::Running {
            if let Err(error) = self.iterate() {
                operation.cancel();
                return Err(error);
            }
        }
        if operation.get_state() != pulse::operation::State::Done {
            bail!(AUDIO_UNAVAILABLE);
        }
        Ok(())
    }
}

impl Drop for PulseConnection {
    fn drop(&mut self) {
        self.context.disconnect();
    }
}

/// Resolve defaults once per segment. A monitor is matched by the server's
/// monitor_of_sink metadata, never by its name, suffix or enumeration order.
pub fn selected_sources(system: bool, microphone: bool) -> Result<Vec<AudioSource>> {
    if !system && !microphone {
        return Ok(Vec::new());
    }
    if !supported() {
        bail!(AUDIO_UNAVAILABLE);
    }
    let mut connection = PulseConnection::new()?;
    let defaults = Rc::new(RefCell::new(None));
    let result = defaults.clone();
    let operation = connection
        .context
        .introspect()
        .get_server_info(move |info| {
            *result.borrow_mut() = Some((
                info.default_sink_name.as_deref().map(str::to_owned),
                info.default_source_name.as_deref().map(str::to_owned),
            ));
        });
    connection.complete(operation)?;
    let (sink, input) = defaults
        .borrow_mut()
        .take()
        .ok_or_else(|| anyhow!(AUDIO_UNAVAILABLE))?;
    let server = connection
        .context
        .get_server()
        .ok_or_else(|| anyhow!(AUDIO_UNAVAILABLE))?;
    let sources = Rc::new(RefCell::new(Vec::new()));
    let result = sources.clone();
    let operation = connection
        .context
        .introspect()
        .get_source_info_list(move |item| {
            if let pulse::callbacks::ListResult::Item(info) = item {
                if let Some(name) = info.name.as_deref() {
                    result.borrow_mut().push(AudioSource {
                        index: info.index,
                        server: server.clone(),
                        name: name.into(),
                        description: info.description.as_deref().unwrap_or(name).into(),
                        monitor_of: info.monitor_of_sink_name.as_deref().map(str::to_owned),
                        muted: info.mute,
                    });
                }
            }
        });
    connection.complete(operation)?;
    let result = select_sources(
        &sources.borrow(),
        sink.as_deref(),
        input.as_deref(),
        system,
        microphone,
    );
    result
}

fn select_sources(
    sources: &[AudioSource],
    sink: Option<&str>,
    input: Option<&str>,
    system: bool,
    microphone: bool,
) -> Result<Vec<AudioSource>> {
    let mut selected = Vec::new();
    if system {
        let monitor = sources
            .iter()
            .find(|source| sink.is_some() && source.monitor_of.as_deref() == sink)
            .ok_or_else(|| anyhow!(AUDIO_UNAVAILABLE))?;
        if monitor.muted {
            bail!(AUDIO_UNAVAILABLE);
        }
        selected.push(monitor.clone());
    }
    if microphone {
        let mic = sources
            .iter()
            .find(|source| Some(source.name.as_str()) == input && source.monitor_of.is_none())
            .ok_or_else(|| anyhow!(MICROPHONE_UNAVAILABLE))?;
        if mic.muted {
            bail!(MICROPHONE_UNAVAILABLE);
        }
        selected.push(mic.clone());
    }
    Ok(selected)
}

fn sample_presentation_time(
    elapsed: Duration,
    latency: pulse::stream::Latency,
) -> Option<Duration> {
    match latency {
        pulse::stream::Latency::Positive(value) => {
            Some(elapsed.saturating_sub(Duration::from_micros(value.0)))
        }
        pulse::stream::Latency::Negative(value) => Some(elapsed + Duration::from_micros(value.0)),
        pulse::stream::Latency::None => None,
    }
}

fn validate_audio_continuity(previous_end: Option<Duration>, next: Duration) -> Result<()> {
    if previous_end.is_some_and(|expected| expected.abs_diff(next) > Duration::from_millis(100)) {
        bail!(AUDIO_RECORDING_FAILED);
    }
    Ok(())
}

fn pcm_bytes_before_cutoff(pts: Duration, cutoff: Duration, bytes: usize) -> usize {
    let frames = cutoff.saturating_sub(pts).as_nanos() * 48_000 / 1_000_000_000;
    bytes.min(
        usize::try_from(frames)
            .unwrap_or(usize::MAX)
            .saturating_mul(8),
    )
}

const PCM_BYTES_PER_SECOND: u64 = 48_000 * 2 * 4;
const PCM_QUEUE_BYTES: u64 = PCM_BYTES_PER_SECOND / 4;

struct NativeInput {
    stream: pulse::stream::Stream,
    source: AudioSource,
    next_pts: Option<Duration>,
    last_data: Instant,
}

/// Explicit async libpulse capture avoids GStreamer's pulsesrc synchronous
/// open/flush waits. The mainloop is only iterated nonblocking on this worker.
/// Native overflow, holes, source movement and server failure all fail closed.
pub struct NativeCapture {
    inputs: Vec<NativeInput>,
    connection: PulseConnection,
    failed: Rc<std::cell::Cell<bool>>,
}

impl NativeCapture {
    fn new(sources: &[AudioSource]) -> Result<Self> {
        let first = sources.first().context(AUDIO_UNAVAILABLE)?;
        let mut connection = PulseConnection::new_at(Some(&first.server))?;
        let failed = Rc::new(std::cell::Cell::new(false));
        let mut inputs = Vec::new();
        let spec = pulse::sample::Spec {
            format: pulse::sample::Format::F32le,
            channels: 2,
            rate: 48_000,
        };
        let attributes = pulse::def::BufferAttr {
            maxlength: PCM_QUEUE_BYTES as u32,
            fragsize: 3_840,
            tlength: u32::MAX,
            prebuf: u32::MAX,
            minreq: u32::MAX,
        };
        for source in sources {
            let mut stream =
                pulse::stream::Stream::new(&mut connection.context, "Kiri recording", &spec, None)
                    .context(AUDIO_UNAVAILABLE)?;
            let overflow = failed.clone();
            stream.set_overflow_callback(Some(Box::new(move || overflow.set(true))));
            let moved = failed.clone();
            stream.set_moved_callback(Some(Box::new(move || moved.set(true))));
            stream
                .connect_record(
                    Some(&source.name),
                    Some(&attributes),
                    pulse::stream::FlagSet::START_CORKED
                        | pulse::stream::FlagSet::ADJUST_LATENCY
                        | pulse::stream::FlagSet::DONT_MOVE
                        | pulse::stream::FlagSet::AUTO_TIMING_UPDATE
                        | pulse::stream::FlagSet::INTERPOLATE_TIMING,
                )
                .map_err(|_| anyhow!(AUDIO_UNAVAILABLE))?;
            while stream.get_state() != pulse::stream::State::Ready {
                if matches!(
                    stream.get_state(),
                    pulse::stream::State::Failed | pulse::stream::State::Terminated
                ) {
                    bail!(AUDIO_UNAVAILABLE);
                }
                connection.iterate()?;
            }
            if stream.get_device_index() != Some(source.index)
                || stream.get_sample_spec() != Some(&spec)
                || stream.get_buffer_attr().is_none_or(|actual| {
                    actual.maxlength > PCM_QUEUE_BYTES as u32
                        || actual.fragsize > PCM_QUEUE_BYTES as u32
                })
            {
                bail!(AUDIO_UNAVAILABLE);
            }
            inputs.push(NativeInput {
                stream,
                source: source.clone(),
                next_pts: None,
                last_data: Instant::now(),
            });
        }
        Ok(Self {
            inputs,
            connection,
            failed,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        self.connection.deadline = Instant::now() + DISCOVERY_TIMEOUT;
        let operations = self
            .inputs
            .iter_mut()
            .map(|input| {
                let failed = self.failed.clone();
                input.last_data = Instant::now();
                input.stream.uncork(Some(Box::new(move |success| {
                    if !success {
                        failed.set(true);
                    }
                })))
            })
            .collect::<Vec<_>>();
        for operation in operations {
            self.connection.complete(operation)?;
        }
        // Refresh after uncork: prepared/corked latency is not a valid anchor.
        let updates = self
            .inputs
            .iter_mut()
            .map(|input| input.stream.update_timing_info(None))
            .collect::<Vec<_>>();
        for update in updates {
            self.connection.complete(update)?;
        }
        self.check()
    }

    pub fn finish(
        &mut self,
        origin: Instant,
        cutoff: Duration,
        mut consume: impl FnMut(usize, Vec<u8>, Duration, Duration) -> Result<()>,
    ) -> Result<()> {
        // Drain source latency only through the frozen video boundary. Future
        // monitor samples and post-stop microphone samples are never submitted.
        let deadline = Instant::now() + Duration::from_millis(500);
        let tolerance = Duration::from_nanos(1_000_000_000 / 48_000 + 1);
        while self
            .inputs
            .iter()
            .any(|input| input.next_pts.is_none_or(|end| end + tolerance < cutoff))
        {
            if Instant::now() >= deadline {
                bail!(AUDIO_RECORDING_FAILED);
            }
            self.pump(origin, Some(cutoff), &mut consume)?;
            std::thread::sleep(Duration::from_millis(2));
        }
        self.check()
    }

    fn check(&self) -> Result<()> {
        if self.failed.get() || self.connection.context.get_state() != pulse::context::State::Ready
        {
            bail!(AUDIO_RECORDING_FAILED);
        }
        for input in &self.inputs {
            if input.stream.get_state() != pulse::stream::State::Ready
                || input.stream.get_device_index() != Some(input.source.index)
                || input.stream.is_suspended().unwrap_or(true)
                || input.last_data.elapsed() > Duration::from_secs(2)
            {
                bail!(AUDIO_RECORDING_FAILED);
            }
        }
        Ok(())
    }

    pub fn pump(
        &mut self,
        origin: Instant,
        cutoff: Option<Duration>,
        mut consume: impl FnMut(usize, Vec<u8>, Duration, Duration) -> Result<()>,
    ) -> Result<()> {
        // Refresh the read-only context deadline; no operation below waits for
        // server I/O. Server loss and stalled data have independent checks.
        self.connection.deadline = Instant::now() + DISCOVERY_TIMEOUT;
        self.connection
            .iterate()
            .map_err(|_| anyhow!(AUDIO_RECORDING_FAILED))?;
        self.check()?;
        for (index, input) in self.inputs.iter_mut().enumerate() {
            for _ in 0..128 {
                let queued = input
                    .stream
                    .readable_size()
                    .context(AUDIO_RECORDING_FAILED)? as u64;
                if queued > PCM_QUEUE_BYTES {
                    bail!(AUDIO_RECORDING_FAILED);
                }
                if queued == 0 {
                    break;
                }
                let elapsed = origin.elapsed();
                let Some(measured) = sample_presentation_time(
                    elapsed,
                    input
                        .stream
                        .get_latency()
                        .map_err(|_| anyhow!(AUDIO_RECORDING_FAILED))?,
                ) else {
                    break;
                };
                validate_audio_continuity(input.next_pts, measured)?;
                // Monitors can deliver future playback. Leave it unread until
                // presentation time, so stopping cannot include future sound.
                if cutoff.is_some_and(|cutoff| measured >= cutoff) {
                    input.next_pts = cutoff;
                    break;
                }
                if measured > elapsed {
                    break;
                }
                let mut bytes = match input
                    .stream
                    .peek()
                    .map_err(|_| anyhow!(AUDIO_RECORDING_FAILED))?
                {
                    pulse::stream::PeekResult::Data(bytes) => bytes.to_vec(),
                    pulse::stream::PeekResult::Empty => break,
                    pulse::stream::PeekResult::Hole(_) => bail!(AUDIO_RECORDING_FAILED),
                };
                if bytes.is_empty() || !bytes.len().is_multiple_of(8) {
                    bail!(AUDIO_RECORDING_FAILED);
                }
                let mut duration =
                    Duration::from_nanos(bytes.len() as u64 * 1_000_000_000 / PCM_BYTES_PER_SECOND);
                if let Some(cutoff) = cutoff {
                    bytes.truncate(pcm_bytes_before_cutoff(measured, cutoff, bytes.len()));
                    duration = Duration::from_nanos(
                        bytes.len() as u64 * 1_000_000_000 / PCM_BYTES_PER_SECOND,
                    );
                    if bytes.is_empty() {
                        input.next_pts = Some(cutoff);
                        break;
                    }
                } else if measured + duration > elapsed {
                    break;
                }
                // Latency already includes the client unread position: never
                // subtract readable_size twice. audiorate smooths small clock
                // corrections; a large native discontinuity fails closed.
                let pts = measured;
                input.next_pts = Some(pts + duration);
                input
                    .stream
                    .discard()
                    .map_err(|_| anyhow!(AUDIO_RECORDING_FAILED))?;
                input.last_data = Instant::now();
                consume(index, bytes, pts, duration)?;
            }
        }
        self.check()
    }
}

impl Drop for NativeCapture {
    fn drop(&mut self) {
        // disconnect queues teardown without flushing or waiting for a server
        // acknowledgement; callbacks and streams die before the mainloop.
        for input in &mut self.inputs {
            let _ = input.stream.disconnect();
        }
    }
}

pub struct RecordingAudio {
    devices: Vec<AudioSource>,
    sources: Vec<gstreamer::Element>,
    progress: Vec<Arc<Mutex<Option<Instant>>>>,
    failed: Arc<AtomicBool>,
    samples: Vec<Arc<AtomicU64>>,
}

impl RecordingAudio {
    /// All identifiers in this description are generated locally. Device names
    /// are assigned as properties afterward, never interpolated into a parser.
    pub fn description(count: usize, source: &str) -> String {
        if count == 0 {
            return String::new();
        }
        let mut description = String::from(" audiomixer name=audio_mix ! audioconvert ! audioresample ! audio/x-raw,format=F32LE,rate=48000,channels=2,layout=interleaved ! avenc_aac bitrate=192000 ! aacparse ! queue max-size-buffers=0 max-size-bytes=262144 max-size-time=250000000 ! mux.audio_0");
        for index in 0..count {
            description.push_str(&format!(" {source} name=audio_{index} ! audioconvert ! audioresample ! audio/x-raw,format=F32LE,rate=48000,channels=2,layout=interleaved ! audiorate tolerance=20000000 skip-to-first=true ! queue name=audio_queue_{index} max-size-buffers=0 max-size-bytes=96000 max-size-time=250000000 ! audio_mix."));
        }
        description
    }

    pub fn attach(pipeline: &gstreamer::Pipeline, sources: &[AudioSource]) -> Result<Self> {
        let mut audio = Self::observe(pipeline, sources.len())?;
        audio.devices = sources.to_vec();
        for source in &audio.sources {
            let appsrc = source
                .clone()
                .downcast::<gstreamer_app::AppSrc>()
                .map_err(|_| anyhow!(AUDIO_UNAVAILABLE))?;
            appsrc.set_max_bytes(PCM_QUEUE_BYTES);
            appsrc.set_block(false);
        }
        Ok(audio)
    }

    pub fn observe(pipeline: &gstreamer::Pipeline, count: usize) -> Result<Self> {
        let failed = Arc::new(AtomicBool::new(false));
        let mut sources = Vec::new();
        let mut progress = Vec::new();
        let mut samples = Vec::new();
        for index in 0..count {
            let source = pipeline
                .by_name(&format!("audio_{index}"))
                .context("Missing audio source")?;
            let queue = pipeline
                .by_name(&format!("audio_queue_{index}"))
                .context("Missing audio queue")?;
            let overloaded = failed.clone();
            queue.connect("overrun", false, move |_| {
                overloaded.store(true, Ordering::Release);
                None
            });
            let last = Arc::new(Mutex::new(None));
            let count = Arc::new(AtomicU64::new(0));
            let update = last.clone();
            let counter = count.clone();
            let discontinuity = failed.clone();
            queue
                .static_pad("sink")
                .context("Missing audio queue pad")?
                .add_probe(gstreamer::PadProbeType::BUFFER, move |_, info| {
                    if let Some(buffer) = info.buffer() {
                        let previous = counter.fetch_add(1, Ordering::AcqRel);
                        if previous > 0 && buffer.flags().contains(gstreamer::BufferFlags::DISCONT)
                        {
                            discontinuity.store(true, Ordering::Release);
                        }
                        *update
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                            Some(Instant::now());
                    }
                    gstreamer::PadProbeReturn::Ok
                });
            sources.push(source);
            progress.push(last);
            samples.push(count);
        }
        Ok(Self {
            devices: Vec::new(),
            sources,
            progress,
            failed,
            samples,
        })
    }

    pub fn prepare_native(&self) -> Result<Option<NativeCapture>> {
        if self.devices.is_empty() {
            Ok(None)
        } else {
            NativeCapture::new(&self.devices).map(Some)
        }
    }

    pub fn push(
        &self,
        index: usize,
        bytes: Vec<u8>,
        pts: Duration,
        duration: Duration,
    ) -> Result<()> {
        let appsrc = self.sources[index]
            .clone()
            .downcast::<gstreamer_app::AppSrc>()
            .map_err(|_| anyhow!(AUDIO_RECORDING_FAILED))?;
        if appsrc.current_level_bytes() + bytes.len() as u64 > PCM_QUEUE_BYTES {
            bail!(AUDIO_RECORDING_FAILED);
        }
        let mut buffer = gstreamer::Buffer::from_mut_slice(bytes);
        {
            let buffer = buffer.get_mut().context(AUDIO_RECORDING_FAILED)?;
            buffer.set_pts(gstreamer::ClockTime::from_nseconds(pts.as_nanos() as u64));
            buffer.set_duration(gstreamer::ClockTime::from_nseconds(
                duration.as_nanos() as u64
            ));
        }
        appsrc.push_buffer(buffer).context(AUDIO_RECORDING_FAILED)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.failed.load(Ordering::Acquire)
            || self
                .samples
                .iter()
                .any(|count| count.load(Ordering::Acquire) == 0)
        {
            bail!(AUDIO_RECORDING_FAILED);
        }
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        !self.sources.is_empty()
    }

    pub fn check(&self, started: Instant) -> Result<()> {
        if self.failed.load(Ordering::Acquire) {
            bail!(AUDIO_RECORDING_FAILED);
        }
        for last in &self.progress {
            let last = *last.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if last.unwrap_or(started).elapsed() > Duration::from_secs(2) {
                bail!(AUDIO_RECORDING_FAILED);
            }
        }
        Ok(())
    }

    pub fn finish(&self) -> Result<()> {
        for source in &self.sources {
            if let Ok(appsrc) = source.clone().downcast::<gstreamer_app::AppSrc>() {
                appsrc.end_of_stream().context(AUDIO_RECORDING_FAILED)?;
            } else if !source.send_event(gstreamer::event::Eos::new()) {
                bail!(AUDIO_RECORDING_FAILED);
            }
        }
        Ok(())
    }
}

/// Same resolved non-monitor source as recording; no PCM is written to disk.
pub fn microphone_check(
    mut current: impl FnMut() -> bool,
    mut level: impl FnMut(String, f32) -> Result<()>,
) -> Result<()> {
    if !current() {
        return Ok(());
    }
    let source = selected_sources(false, true)?
        .pop()
        .context(MICROPHONE_UNAVAILABLE)?;
    let mut capture = NativeCapture::new(std::slice::from_ref(&source))?;
    if !current() {
        return Ok(());
    }
    let started = Instant::now();
    capture.start()?;
    let mut peak = 0.0f32;
    let mut last_meter = Instant::now();
    while current() && started.elapsed() < Duration::from_secs(5) {
        capture.pump(started, None, |_, bytes, _, _| {
            for chunk in bytes.as_chunks::<4>().0 {
                let value = f32::from_le_bytes(*chunk);
                if value.is_finite() {
                    peak = peak.max(value.abs());
                }
            }
            Ok(())
        })?;
        if last_meter.elapsed() >= Duration::from_millis(100) {
            level(source.description.clone(), peak)?;
            peak = 0.0;
            last_meter = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(name: &str, monitor: Option<&str>) -> AudioSource {
        AudioSource {
            name: name.into(),
            index: 0,
            server: "unix:synthetic-test-only".into(),
            description: name.into(),
            monitor_of: monitor.map(str::to_owned),
            muted: false,
        }
    }
    #[test]
    fn audio_never_connects_to_network_or_ambiguous_servers() {
        assert!(validate_local_server("unix:/run/user/1000/pulse/native").is_ok());
        assert!(validate_local_server("/tmp/private-pulse/native").is_ok());
        for server in [
            "tcp:host",
            "host",
            "unix:/tmp/socket tcp:host",
            "{host}unix:/tmp/socket",
            "unix:relative",
        ] {
            assert!(validate_local_server(server).is_err());
        }
    }

    #[test]
    fn signed_latency_and_stop_cutoff_preserve_monitor_timing() {
        use pulse::{stream::Latency, time::MicroSeconds};
        let now = Duration::from_millis(100);
        assert_eq!(
            sample_presentation_time(now, Latency::Positive(MicroSeconds(20_000))),
            Some(Duration::from_millis(80))
        );
        assert_eq!(
            sample_presentation_time(now, Latency::Negative(MicroSeconds(20_000))),
            Some(Duration::from_millis(120))
        );
        assert_eq!(sample_presentation_time(now, Latency::None), None);
        assert_eq!(
            pcm_bytes_before_cutoff(Duration::from_millis(90), now, 7680),
            3840
        );
        assert_eq!(
            pcm_bytes_before_cutoff(Duration::from_millis(120), now, 7680),
            0
        );
        assert_eq!(
            pcm_bytes_before_cutoff(Duration::from_millis(80), now, 3840),
            3840
        );
    }

    #[test]
    fn final_cutoff_cannot_hide_a_forward_clock_jump() {
        assert!(validate_audio_continuity(
            Some(Duration::from_millis(500)),
            Duration::from_millis(1100)
        )
        .is_err());
        assert!(validate_audio_continuity(
            Some(Duration::from_millis(500)),
            Duration::from_millis(510)
        )
        .is_ok());
        assert!(validate_audio_continuity(None, Duration::from_millis(10)).is_ok());
    }

    #[test]
    fn selection_never_substitutes_default_input_for_output_monitor() {
        let sources = [
            source("mic", None),
            source("arbitrary monitor name", Some("speakers")),
            source("other.monitor", Some("headphones")),
        ];
        assert_eq!(
            select_sources(&sources, Some("speakers"), Some("mic"), true, false).unwrap()[0].name,
            "arbitrary monitor name"
        );
        assert_eq!(
            select_sources(&sources, Some("speakers"), Some("mic"), false, true).unwrap()[0].name,
            "mic"
        );
        assert_eq!(
            select_sources(&sources, Some("speakers"), Some("mic"), true, true)
                .unwrap()
                .len(),
            2
        );
        assert!(select_sources(&sources, Some("missing"), Some("mic"), true, false).is_err());
        assert!(select_sources(
            &sources,
            Some("speakers"),
            Some("other.monitor"),
            false,
            true
        )
        .is_err());
    }
    #[test]
    fn muted_or_absent_input_is_not_silently_recorded() {
        let mut mic = source("mic", None);
        mic.muted = true;
        assert!(select_sources(&[mic], None, Some("mic"), false, true).is_err());
        assert!(select_sources(&[], None, None, false, true).is_err());
        assert!(selected_sources(false, false).unwrap().is_empty());
        assert!(RecordingAudio::description(0, "pulsesrc").is_empty());
    }
}
