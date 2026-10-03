//! Bounded native Linux cuts. Decode accurate source ranges, then feed one
//! H.264/AAC encoder in timeline order. No compressed-stream keyframe cuts and
//! no per-clip AAC encoder delay. All pipelines are stopped on every exit path.

use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use gstreamer as gst;
use gstreamer_app::{AppSink, AppSrc};
use gstreamer_pbutils::prelude::*;

use super::{
    ExportProgressRange, PreparedVideoAnnotation, VideoEffect, VideoExportPreset, VideoSegment,
};

const POLL: gst::ClockTime = gst::ClockTime::from_mseconds(20);
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
const AUDIO_RATE: u64 = 48_000;
// The selected libav AAC-LC encoder declares 1024 samples of initial padding.
// GStreamer's avenc bridge emits them without shifting the encoded timeline;
// compensate at the mux pad so the edit list retains decoder preroll without
// shifting audible content or discarding the final source samples.
const AAC_PRIMING_FRAMES: u64 = 1024;
const AUDIO_FRAME_BYTES: usize = 4; // S16LE, stereo
const AUDIO_QUEUE_BYTES: u64 = AUDIO_RATE * AUDIO_FRAME_BYTES as u64;
const WORKER_STOPPED: &str = "Linux video export worker stopped";

struct Pipeline(gst::Pipeline);
impl std::ops::Deref for Pipeline {
    type Target = gst::Pipeline;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Drop for Pipeline {
    fn drop(&mut self) {
        let _ = self.0.set_state(gst::State::Null);
    }
}

fn pipeline(description: &str) -> Result<Pipeline> {
    gst::init()?;
    let element = gst::parse::launch(description)
        .context("Could not prepare Linux video export; check the installed GStreamer plugins")?;
    Ok(Pipeline(element.downcast::<gst::Pipeline>().map_err(
        |_| anyhow!("Linux video export did not create a pipeline"),
    )?))
}

fn location(pipeline: &gst::Pipeline, name: &str, path: &Path) -> Result<()> {
    pipeline
        .by_name(name)
        .context("The media file element is missing")?
        .set_property(
            "location",
            path.to_str().context("The media path is not UTF-8")?,
        );
    Ok(())
}

fn bus(pipeline: &gst::Pipeline) -> Result<gst::Bus> {
    pipeline
        .bus()
        .context("The Linux video pipeline has no bus")
}

fn check_bus(bus: &gst::Bus) -> Result<()> {
    if let Some(message) = bus.pop_filtered(&[gst::MessageType::Error]) {
        if let gst::MessageView::Error(error) = message.view() {
            bail!(
                "Linux video export failed: {} ({})",
                error.error(),
                error.debug().as_deref().unwrap_or("no details")
            );
        }
    }
    Ok(())
}

fn check(progress: &ExportProgressRange, stopped: &AtomicBool, bus: &gst::Bus) -> Result<()> {
    progress.check()?;
    if stopped.load(Ordering::Acquire) {
        bail!(WORKER_STOPPED);
    }
    check_bus(bus)
}

fn ns(seconds: f64) -> Result<u64> {
    if !seconds.is_finite() || seconds < 0.0 || seconds >= u64::MAX as f64 / 1e9 {
        bail!("Video time is outside the supported range");
    }
    Ok((seconds * 1e9).round() as u64)
}
fn clock(value: u64) -> gst::ClockTime {
    gst::ClockTime::from_nseconds(value)
}
fn audio_time(frame: u64) -> u64 {
    (u128::from(frame) * 1_000_000_000 / u128::from(AUDIO_RATE)) as u64
}
fn audio_frame(time: u64) -> u64 {
    (u128::from(time) * u128::from(AUDIO_RATE) / 1_000_000_000) as u64
}

pub(super) fn available() -> bool {
    gst::init().is_ok()
        && [
            "filesrc",
            "decodebin",
            "videoconvert",
            "videoflip",
            "videoscale",
            "appsink",
            "appsrc",
            "x264enc",
            "mp4mux",
            "filesink",
            "audioconvert",
            "audioresample",
            "avenc_aac",
            "aacparse",
            "queue",
            "capsfilter",
        ]
        .iter()
        .all(|name| gst::ElementFactory::find(name).is_some())
}

struct Source {
    width: u32,
    height: u32,
    duration: f64,
    audio: bool,
}

fn inspect(path: &Path) -> Result<Source> {
    gst::init()?;
    let discoverer = gstreamer_pbutils::Discoverer::new(gst::ClockTime::from_seconds(5))?;
    let info = discoverer.discover_uri(&glib::filename_to_uri(path, None)?)?;
    if info.result() != gstreamer_pbutils::DiscovererResult::Ok {
        bail!("The video could not be completely inspected");
    }
    let streams = info.video_streams();
    if streams.len() != 1 || info.audio_streams().len() > 1 || !info.subtitle_streams().is_empty() {
        bail!("Linux export requires one video track, at most one audio track, and no subtitle tracks");
    }
    let stream = &streams[0];
    if stream.par() != gst::Fraction::new(1, 1) {
        bail!("Linux editing requires square-pixel video; anamorphic input is not supported");
    }
    if stream.is_interlaced() {
        bail!("Interlaced video is not supported by Linux editing");
    }
    let orientation = stream.tags().and_then(|tags| {
        tags.get::<gst::tags::ImageOrientation>()
            .map(|value| value.get().to_owned())
    });
    let (width, height) = if matches!(
        orientation.as_deref(),
        Some("rotate-90" | "rotate-270" | "flip-rotate-90" | "flip-rotate-270")
    ) {
        (stream.height(), stream.width())
    } else {
        (stream.width(), stream.height())
    };
    // Keep decoded frames and all appsrc/appsink queues bounded even for a
    // hand-written IPC request. This matches the editor's pixel budget.
    if width < 2
        || height < 2
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 32_000_000
    {
        bail!("Video dimensions exceed the Linux editing limit");
    }
    let duration = info
        .duration()
        .context("Video duration is unavailable")?
        .nseconds() as f64
        / 1e9;
    if !duration.is_finite() || duration <= 0.0 {
        bail!("Video duration is invalid");
    }
    Ok(Source {
        width,
        height,
        duration,
        audio: !info.audio_streams().is_empty(),
    })
}

fn dimensions(source: &Source, preset: VideoExportPreset) -> (u32, u32) {
    let max = preset.max_edge();
    let ratio = if max == 0 {
        1.0
    } else {
        (f64::from(max) / f64::from(source.width.max(source.height))).min(1.0)
    };
    // H.264 I420 needs even dimensions. Never enlarge a small source.
    (
        ((f64::from(source.width) * ratio).floor() as u32 / 2 * 2).max(2),
        ((f64::from(source.height) * ratio).floor() as u32 / 2 * 2).max(2),
    )
}

fn decoder(
    source: &Path,
    conversion: &str,
    segment: &VideoSegment,
    progress: &ExportProgressRange,
    stopped: &AtomicBool,
    preceding_keyframe: bool,
) -> Result<(Pipeline, AppSink)> {
    let pipe = pipeline(&format!("filesrc name=input ! decodebin ! {conversion} ! appsink name=sink max-buffers=2 drop=false sync=false"))?;
    location(&pipe, "input", source)?;
    let sink = pipe
        .by_name("sink")
        .context("The decoder sink is missing")?
        .downcast::<AppSink>()
        .map_err(|_| anyhow!("The decoder sink has the wrong type"))?;
    let bus = bus(&pipe)?;
    pipe.set_state(gst::State::Paused)
        .context("Could not prepare the Linux video decoder")?;
    let deadline = Instant::now() + STALL_TIMEOUT;
    loop {
        check(progress, stopped, &bus)?;
        let (result, state, _) = pipe.state(POLL);
        result.context("Could not prepare the source video for seeking")?;
        if state == gst::State::Paused {
            break;
        }
        if Instant::now() >= deadline {
            bail!("The video decoder timed out before seeking");
        }
    }
    // Video starts at the preceding keyframe so our one-frame lookahead can
    // retain a long-held frame even when a decoder reports its nominal duration.
    // This is a raw decode/re-encode, never a compressed keyframe-only cut.
    let mut flags = gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE;
    if preceding_keyframe {
        flags |= gst::SeekFlags::KEY_UNIT | gst::SeekFlags::SNAP_BEFORE;
    }
    pipe.seek(
        1.0,
        flags,
        gst::SeekType::Set,
        clock(ns(segment.start)?),
        gst::SeekType::Set,
        clock(ns(segment.end)?),
    )
    .context("This source does not support accurate Linux video cuts")?;
    check(progress, stopped, &bus)?;
    pipe.set_state(gst::State::Playing)
        .context("Could not start Linux video decoding")?;
    Ok((pipe, sink))
}

fn push(
    src: &AppSrc,
    buffer: gst::Buffer,
    limit: u64,
    progress: &ExportProgressRange,
    stopped: &AtomicBool,
    bus: &gst::Bus,
) -> Result<()> {
    let deadline = Instant::now() + STALL_TIMEOUT;
    let size = buffer.size() as u64;
    if size > limit {
        bail!("A decoded media buffer exceeds the export queue limit");
    }
    while src.current_level_bytes().saturating_add(size) > limit {
        check(progress, stopped, bus)?;
        if Instant::now() >= deadline {
            bail!("Linux video encoding stopped accepting media");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    check(progress, stopped, bus)?;
    src.push_buffer(buffer)
        .context("The Linux encoder rejected decoded media")?;
    Ok(())
}

fn sample_time(sample: &gst::Sample) -> Result<u64> {
    let pts = sample
        .buffer()
        .and_then(|buffer| buffer.pts())
        .context("A decoded media sample has no timestamp")?;
    let segment = sample
        .segment()
        .and_then(|segment| segment.downcast_ref::<gst::ClockTime>())
        .context("A decoded media sample has no time segment")?;
    segment
        .to_stream_time(pts)
        .map(|time| time.nseconds())
        .context("A decoded media timestamp is outside its segment")
}

struct VideoOutput<'a> {
    src: &'a AppSrc,
    width: u32,
    height: u32,
    total: u64,
}

fn render_video(
    source: &Path,
    segments: &[VideoSegment],
    output: VideoOutput<'_>,
    progress: &ExportProgressRange,
    stopped: &AtomicBool,
    output_bus: &gst::Bus,
) -> Result<()> {
    let mut offset = 0u64;
    let VideoOutput {
        src,
        width,
        height,
        total,
    } = output;
    let limit = u64::from(width) * u64::from(height) * 4 * 2;
    for segment in segments {
        check(progress, stopped, output_bus)?;
        let start = ns(segment.start)?;
        let end = ns(segment.end)?;
        let (pipe, sink) = decoder(
            source,
            &format!(
                "video/x-raw ! videoconvert ! videoflip video-direction=auto ! videoscale ! \
             video/x-raw,format=BGRA,width={width},height={height},pixel-aspect-ratio=1/1"
            ),
            segment,
            progress,
            stopped,
            true,
        )?;
        let input_bus = bus(&pipe)?;
        let mut last = Instant::now();
        let mut covered = 0u64;
        let mut pending: Option<(gst::Sample, u64)> = None;
        loop {
            check(progress, stopped, output_bus)?;
            check_bus(&input_bus)?;
            let next = if let Some(sample) = sink.try_pull_sample(POLL) {
                last = Instant::now();
                let time = sample_time(&sample)?;
                Some((sample, time))
            } else if sink.is_eos() {
                None
            } else {
                if last.elapsed() > STALL_TIMEOUT {
                    bail!("Linux video decoding stopped delivering frames");
                }
                continue;
            };
            if let Some((sample, time)) = pending.take() {
                let next_time = next.as_ref().map_or(end, |(_, time)| *time);
                if next_time <= time {
                    bail!("The source has non-increasing video timestamps");
                }
                let from = time.max(start);
                let to = next_time.min(end);
                if to > from {
                    if from > start + covered + 1_000_000 {
                        bail!("The source contains a gap before its first video frame");
                    }
                    let mut buffer = sample
                        .buffer()
                        .context("The decoded video buffer is missing")?
                        .copy();
                    let buffer_ref = buffer.make_mut();
                    buffer_ref.set_pts(clock(offset + from - start));
                    buffer_ref.set_dts(None);
                    buffer_ref.set_duration(clock(to - from));
                    buffer_ref.set_offset(gst::format::Buffers::OFFSET_NONE);
                    buffer_ref.set_offset_end(gst::format::Buffers::OFFSET_NONE);
                    push(src, buffer, limit, progress, stopped, output_bus)?;
                    covered = covered.max(to - start);
                    progress.report((offset + covered) as f64 / total as f64 * 0.95);
                }
            }
            match next {
                Some(value) => pending = Some(value),
                None => break,
            }
        }
        check_bus(&input_bus)?;
        if covered == 0 || covered.abs_diff(end - start) > 1_000_000 {
            bail!(
                "Decoded video did not cover the complete requested cut ({covered} of {} ns)",
                end - start
            );
        }
        offset = offset
            .checked_add(end - start)
            .context("Video timeline is too long")?;
    }
    src.end_of_stream()
        .context("Could not finish the Linux video stream")?;
    Ok(())
}

fn push_audio(
    src: &AppSrc,
    bytes: Vec<u8>,
    start: u64,
    progress: &ExportProgressRange,
    stopped: &AtomicBool,
    output_bus: &gst::Bus,
) -> Result<u64> {
    let frames = bytes.len() as u64 / AUDIO_FRAME_BYTES as u64;
    let mut buffer = gst::Buffer::from_mut_slice(bytes);
    let buffer_ref = buffer.make_mut();
    buffer_ref.set_pts(clock(audio_time(start)));
    buffer_ref.set_duration(clock(audio_time(start + frames) - audio_time(start)));
    push(
        src,
        buffer,
        AUDIO_QUEUE_BYTES,
        progress,
        stopped,
        output_bus,
    )?;
    Ok(start + frames)
}

fn silence(
    src: &AppSrc,
    cursor: &mut u64,
    target: u64,
    progress: &ExportProgressRange,
    stopped: &AtomicBool,
    output_bus: &gst::Bus,
) -> Result<()> {
    while *cursor < target {
        let frames = (target - *cursor).min(4096);
        *cursor = push_audio(
            src,
            vec![0; frames as usize * AUDIO_FRAME_BYTES],
            *cursor,
            progress,
            stopped,
            output_bus,
        )?;
    }
    Ok(())
}

fn render_audio(
    source: &Path,
    segments: &[VideoSegment],
    src: &AppSrc,
    progress: &ExportProgressRange,
    stopped: &AtomicBool,
    output_bus: &gst::Bus,
) -> Result<()> {
    let mut offset = 0u64;
    let mut cursor = 0u64;
    for segment in segments {
        let start = ns(segment.start)?;
        let end = ns(segment.end)?;
        let output_end = audio_frame(offset + end - start);
        let (pipe, sink) = decoder(source,
            "audio/x-raw ! audioconvert ! audioresample ! audio/x-raw,format=S16LE,layout=interleaved,channels=2,rate=48000",
            segment, progress, stopped, false)?;
        let input_bus = bus(&pipe)?;
        let mut last = Instant::now();
        loop {
            check(progress, stopped, output_bus)?;
            check_bus(&input_bus)?;
            if let Some(sample) = sink.try_pull_sample(POLL) {
                last = Instant::now();
                let time = sample_time(&sample)?;
                let mapped = sample
                    .buffer()
                    .context("The decoded audio buffer is missing")?
                    .map_readable()?;
                if mapped.len() % AUDIO_FRAME_BYTES != 0 {
                    bail!("Decoded audio is not complete stereo samples");
                }
                let frames = mapped.len() / AUDIO_FRAME_BYTES;
                let skip = if time < start {
                    audio_frame(start - time) as usize
                } else {
                    0
                };
                if skip >= frames {
                    continue;
                }
                let at = audio_frame(offset + time.saturating_sub(start));
                let at = if time < start {
                    audio_frame(offset)
                } else {
                    at
                };
                // Preserve source gaps and delayed audio as silence. Use the
                // cumulative output sample clock so repeated cuts cannot drift.
                silence(
                    src,
                    &mut cursor,
                    at.min(output_end),
                    progress,
                    stopped,
                    output_bus,
                )?;
                let overlap = cursor.saturating_sub(at) as usize;
                let first = skip.saturating_add(overlap).min(frames);
                let count = (frames - first).min(output_end.saturating_sub(cursor) as usize);
                if count > 0 {
                    cursor = push_audio(
                        src,
                        mapped[first * AUDIO_FRAME_BYTES..(first + count) * AUDIO_FRAME_BYTES]
                            .to_vec(),
                        cursor,
                        progress,
                        stopped,
                        output_bus,
                    )?;
                }
            } else if sink.is_eos() {
                break;
            } else if last.elapsed() > STALL_TIMEOUT {
                bail!("Linux audio decoding stopped delivering samples");
            }
        }
        check_bus(&input_bus)?;
        silence(src, &mut cursor, output_end, progress, stopped, output_bus)?;
        offset += end - start;
    }
    src.end_of_stream()
        .context("Could not finish the Linux audio stream")?;
    Ok(())
}

fn wait_for_output(output_bus: &gst::Bus, progress: &ExportProgressRange) -> Result<()> {
    let deadline = Instant::now() + STALL_TIMEOUT;
    loop {
        progress.check()?;
        if let Some(message) =
            output_bus.timed_pop_filtered(POLL, &[gst::MessageType::Eos, gst::MessageType::Error])
        {
            match message.view() {
                gst::MessageView::Eos(_) => break,
                gst::MessageView::Error(error) => {
                    bail!("Linux MP4 finalization failed: {}", error.error())
                }
                _ => {}
            }
        }
        if Instant::now() >= deadline {
            bail!("Linux MP4 finalization timed out");
        }
    }
    Ok(())
}

pub(super) fn platform_export(
    source: &Path,
    output: &Path,
    segments: &[VideoSegment],
    effects: &[VideoEffect],
    annotations: &[PreparedVideoAnnotation],
    preset: VideoExportPreset,
    progress: &ExportProgressRange,
) -> Result<(i64, i64, f64)> {
    progress.check()?;
    if !effects.is_empty()
        || !annotations.is_empty()
        || segments.iter().any(|clip| clip.speed != 1.0)
    {
        bail!("Linux export supports normal-speed cuts only; speed changes, effects, privacy masks, annotations and stickers are not supported");
    }
    if !available() {
        bail!("Linux video export requires the installed GStreamer video and AAC encoding plugins");
    }
    let metadata = inspect(source)?;
    progress.check()?;
    let segments = super::validate_segments(segments, Some(metadata.duration))?;
    let total = segments.iter().try_fold(0u64, |sum, segment| {
        sum.checked_add(ns(segment.end)? - ns(segment.start)?)
            .context("Video timeline is too long")
    })?;
    if total == 0 {
        bail!("The video cut is shorter than one nanosecond");
    }
    let (width, height) = dimensions(&metadata, preset);
    // The shared wrapper later moves this file out of its private staging
    // directory. Preserve owner-only permissions during that import interval.
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(output)
        .context("Could not create a private Linux video export")?;
    let audio = if metadata.audio {
        " appsrc name=audio format=time is-live=false block=false max-bytes=192000 ! \
          audioconvert ! audio/x-raw,format=F32LE ! avenc_aac bitrate=192000 ! audio/mpeg,mpegversion=4,base-profile=lc ! aacparse ! queue max-size-buffers=8 max-size-bytes=0 max-size-time=0 ! mux.audio_0"
    } else {
        ""
    };
    let pipe = pipeline(&format!(
        "appsrc name=video format=time is-live=false block=false ! videoconvert ! video/x-raw,format=I420 ! \
         x264enc tune=zerolatency speed-preset=fast bitrate=12000 key-int-max=120 ! \
         video/x-h264,stream-format=avc,alignment=au ! queue max-size-buffers=4 max-size-bytes=0 max-size-time=0 ! mux.video_0 \
         mp4mux name=mux ! filesink name=output{audio}"))?;
    location(&pipe, "output", output)?;
    let video = pipe
        .by_name("video")
        .context("The video encoder input is missing")?
        .downcast::<AppSrc>()
        .map_err(|_| anyhow!("The video encoder input is invalid"))?;
    video.set_caps(Some(
        &gst::Caps::builder("video/x-raw")
            .field("format", "BGRA")
            .field("width", width as i32)
            .field("height", height as i32)
            .field("framerate", gst::Fraction::new(0, 1))
            .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
            .build(),
    ));
    video.set_max_bytes(u64::from(width) * u64::from(height) * 8);
    let audio = if metadata.audio {
        let audio = pipe
            .by_name("audio")
            .context("The audio encoder input is missing")?
            .downcast::<AppSrc>()
            .map_err(|_| anyhow!("The audio encoder input is invalid"))?;
        audio.set_caps(Some(
            &gst::Caps::builder("audio/x-raw")
                .field("format", "S16LE")
                .field("layout", "interleaved")
                .field("channels", 2i32)
                .field("channel-mask", gst::Bitmask::new(3))
                .field("rate", AUDIO_RATE as i32)
                .build(),
        ));
        Some(audio)
    } else {
        None
    };
    if metadata.audio {
        pipe.by_name("mux")
            .context("The MP4 muxer is missing")?
            .static_pad("audio_0")
            .context("The MP4 audio pad is missing")?
            .set_offset(-(audio_time(AAC_PRIMING_FRAMES) as i64));
    }
    let output_bus = bus(&pipe)?;
    let stopped = AtomicBool::new(false);
    pipe.set_state(gst::State::Playing)
        .context("Could not start Linux MP4 encoding")?;
    // Independent bounded workers prevent an audio/video demux or mux queue
    // from deadlocking when a static frame spans a long stretch of sound.
    let (video_result, audio_result) = std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let result = render_video(
                source,
                &segments,
                VideoOutput {
                    src: &video,
                    width,
                    height,
                    total,
                },
                progress,
                &stopped,
                &output_bus,
            );
            if result.is_err() {
                stopped.store(true, Ordering::Release);
            }
            result
        });
        let audio_result = if let Some(audio) = audio.as_ref() {
            render_audio(source, &segments, audio, progress, &stopped, &output_bus)
        } else {
            Ok(())
        };
        if audio_result.is_err() {
            stopped.store(true, Ordering::Release);
        }
        let video_result = worker
            .join()
            .unwrap_or_else(|_| Err(anyhow!("The Linux video export worker panicked")));
        (video_result, audio_result)
    });
    progress.check()?;
    if let Err(error) = &audio_result {
        if error.to_string() != WORKER_STOPPED {
            return audio_result.map(|_| unreachable!());
        }
    }
    video_result?;
    audio_result?;
    wait_for_output(&output_bus, progress)?;
    drop(pipe);
    progress.check()?;
    let result = inspect(output)?;
    if (result.width, result.height) != (width, height)
        || result.audio != metadata.audio
        || (result.duration - total as f64 / 1e9).abs() > 0.01
    {
        bail!("The exported MP4 does not match the requested dimensions, duration or audio tracks");
    }
    progress.report(1.0);
    Ok((i64::from(width), i64::from(height), result.duration))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video_export::{
        export_video_with_annotations_controlled, ExportControl, ExportPhase, VideoAnnotation,
        VideoEffectKind, EXPORT_CANCELLED,
    };
    use sha2::{Digest, Sha256};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex, OnceLock};

    static NATIVE_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct Evidence(tempfile::TempDir, &'static str);
    impl Evidence {
        fn new(name: &'static str) -> Self {
            Self(tempfile::tempdir().unwrap(), name)
        }
        fn path(&self) -> &Path {
            self.0.path()
        }
    }
    impl Drop for Evidence {
        fn drop(&mut self) {
            let Some(root) = std::env::var_os("KIRI_LINUX_VIDEO_QA_DIR") else {
                return;
            };
            let destination = PathBuf::from(root).join(self.1);
            if std::fs::create_dir_all(&destination).is_err() {
                return;
            }
            let _ = std::fs::write(
                destination.join("test-status.txt"),
                if std::thread::panicking() {
                    "failed\n"
                } else {
                    "passed\n"
                },
            );
            for entry in std::fs::read_dir(self.path()).unwrap().flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_file()) {
                    let _ = std::fs::copy(entry.path(), destination.join(entry.file_name()));
                }
            }
        }
    }

    fn finish(pipe: &Pipeline) {
        let message = bus(pipe)
            .unwrap()
            .timed_pop_filtered(
                gst::ClockTime::from_seconds(20),
                &[gst::MessageType::Eos, gst::MessageType::Error],
            )
            .expect("fixture pipeline timed out");
        if let gst::MessageView::Error(error) = message.view() {
            panic!("{} {:?}", error.error(), error.debug());
        }
        assert!(matches!(message.view(), gst::MessageView::Eos(_)));
    }

    fn fixture(path: &Path, audio: bool, variable: bool) {
        fixture_audio_range(path, audio.then_some(0..400), variable);
    }

    fn fixture_audio_range(
        path: &Path,
        audio_blocks: Option<std::ops::Range<u64>>,
        variable: bool,
    ) {
        let audio = audio_blocks.is_some();
        let audio_branch = if audio {
            " appsrc name=audio format=time ! voaacenc ! aacparse ! queue ! mux.audio_0"
        } else {
            ""
        };
        let pipe = pipeline(&format!("appsrc name=video format=time ! videoconvert ! video/x-raw,format=I420 ! \
            x264enc tune=zerolatency speed-preset=superfast option-string=scenecut=0:keyint=100:min-keyint=100 ! \
            video/x-h264,stream-format=avc,alignment=au ! queue ! mux.video_0 mp4mux name=mux ! filesink name=output{audio_branch}")).unwrap();
        location(&pipe, "output", path).unwrap();
        let video = pipe.by_name("video").unwrap().downcast::<AppSrc>().unwrap();
        video.set_caps(Some(
            &gst::Caps::builder("video/x-raw")
                .field("format", "BGRA")
                .field("width", 64i32)
                .field("height", 48i32)
                .field("framerate", gst::Fraction::new(10, 1))
                .build(),
        ));
        let audio = audio.then(|| {
            let src = pipe.by_name("audio").unwrap().downcast::<AppSrc>().unwrap();
            src.set_caps(Some(
                &gst::Caps::builder("audio/x-raw")
                    .field("format", "S16LE")
                    .field("layout", "interleaved")
                    .field("channels", 2i32)
                    .field("channel-mask", gst::Bitmask::new(3))
                    .field("rate", 48_000i32)
                    .build(),
            ));
            src
        });
        pipe.set_state(gst::State::Playing).unwrap();
        let times = if variable {
            vec![0, 200, 1_300, 1_350, 3_000, 4_000]
        } else {
            (0..=40).map(|index| index * 100).collect()
        };
        for (index, pair) in times.windows(2).enumerate() {
            let color = if variable {
                [
                    [0u8, 0, 255, 255],
                    [0, 255, 0, 255],
                    [255, 0, 0, 255],
                    [255, 255, 255, 255],
                    [0, 255, 255, 255],
                ][index]
            } else {
                [
                    [0u8, 0, 255, 255],
                    [0, 255, 0, 255],
                    [255, 0, 0, 255],
                    [255, 255, 255, 255],
                ][(pair[0] / 1000) as usize]
            };
            let mut buffer = gst::Buffer::from_mut_slice(color.repeat(64 * 48));
            buffer.make_mut().set_pts(clock(pair[0] * 1_000_000));
            buffer
                .make_mut()
                .set_duration(clock((pair[1] - pair[0]) * 1_000_000));
            video.push_buffer(buffer).unwrap();
        }
        video.end_of_stream().unwrap();
        if let Some(audio) = audio {
            for block in audio_blocks.unwrap() {
                let start = block * 480;
                let samples: Vec<_> = (start..start + 480)
                    .flat_map(|index| {
                        let frequency = [220.0, 440.0, 660.0, 880.0][(index / 48_000) as usize];
                        let value = ((index as f64 * frequency * std::f64::consts::TAU / 48_000.0)
                            .sin()
                            * 12_000.0) as i16;
                        [value.to_le_bytes(), value.to_le_bytes()].concat()
                    })
                    .collect();
                let mut buffer = gst::Buffer::from_mut_slice(samples);
                buffer.make_mut().set_pts(clock(audio_time(start)));
                buffer.make_mut().set_duration(clock(10_000_000));
                audio.push_buffer(buffer).unwrap();
            }
            audio.end_of_stream().unwrap();
        }
        finish(&pipe);
    }

    fn decode(path: &Path, audio: bool) -> Vec<(f64, Vec<u8>)> {
        let conversion = if audio {
            "audio/x-raw ! audioconvert ! audioresample ! audio/x-raw,format=S16LE,layout=interleaved,channels=2,rate=48000"
        } else {
            "video/x-raw ! videoconvert ! video/x-raw,format=RGBA"
        };
        let pipe = pipeline(&format!("filesrc name=input ! decodebin ! {conversion} ! appsink name=sink sync=false max-buffers=2")).unwrap();
        location(&pipe, "input", path).unwrap();
        let sink = pipe.by_name("sink").unwrap().downcast::<AppSink>().unwrap();
        pipe.set_state(gst::State::Playing).unwrap();
        let mut result = Vec::new();
        loop {
            if let Some(sample) = sink.try_pull_sample(gst::ClockTime::from_seconds(10)) {
                let buffer = sample.buffer().unwrap();
                result.push((
                    sample_time(&sample).unwrap() as f64 / 1e9,
                    buffer.map_readable().unwrap().to_vec(),
                ));
            } else {
                check_bus(&bus(&pipe).unwrap()).unwrap();
                assert!(sink.is_eos(), "all output frames must decode");
                break;
            }
        }
        result
    }

    fn export(source: &Path, destination: &Path, segments: &[VideoSegment]) -> (i64, i64, f64) {
        let (file, width, height, duration) = export_video_with_annotations_controlled(
            source,
            segments,
            &[],
            &[],
            VideoExportPreset::Original,
            &ExportControl::default(),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o077,
            0,
            "completed exports must stay private before library import"
        );
        std::fs::rename(file, destination).unwrap();
        (width, height, duration)
    }
    fn clip(start: f64, end: f64) -> VideoSegment {
        VideoSegment {
            start,
            end,
            speed: 1.0,
        }
    }
    fn hash(path: &Path) -> String {
        format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
    }
    fn color_at(frames: &[(f64, Vec<u8>)], time: f64) -> [u8; 3] {
        let pixel = &frames
            .iter()
            .rev()
            .find(|(pts, _)| *pts <= time + 1e-6)
            .unwrap()
            .1;
        [pixel[0], pixel[1], pixel[2]]
    }
    fn assert_color(pixel: [u8; 3], expected: [u8; 3]) {
        assert!(
            pixel
                .iter()
                .zip(expected)
                .all(|(a, b)| (i16::from(*a) - i16::from(b)).abs() < 25),
            "{pixel:?} != {expected:?}"
        );
    }
    fn frequency(samples: &[(f64, Vec<u8>)], from: f64, to: f64) -> f64 {
        let values: Vec<i16> = samples
            .iter()
            .flat_map(|(pts, bytes)| {
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, pair)| {
                        let time = pts + index as f64 / 48_000.0;
                        (time >= from && time < to).then(|| i16::from_le_bytes([pair[0], pair[1]]))
                    })
            })
            .collect();
        assert!(values.len() > 4000);
        assert!(
            values.iter().any(|value| value.unsigned_abs() > 5000),
            "audio was lost or replaced by silence"
        );
        let crossings = values
            .windows(2)
            .filter(|pair| pair[0] < 0 && pair[1] >= 0)
            .count();
        crossings as f64 / (values.len() as f64 / 48_000.0)
    }

    fn audio_duration(samples: &[(f64, Vec<u8>)]) -> f64 {
        samples.last().map_or(0.0, |(time, bytes)| {
            time + bytes.len() as f64 / AUDIO_FRAME_BYTES as f64 / 48_000.0
        })
    }

    fn transition_time(samples: &[(f64, Vec<u8>)], expected: f64) -> f64 {
        (0..=80)
            .map(|index| expected - 0.1 + index as f64 * 0.0025)
            .min_by(|a, b| {
                (frequency(samples, a - 0.05, a + 0.05) - 440.0)
                    .abs()
                    .total_cmp(&(frequency(samples, b - 0.05, b + 0.05) - 440.0).abs())
            })
            .unwrap()
    }

    /// Exercise the recording pipeline's actual avenc_aac priming/edit-list
    /// output, including pause merging, rather than only imported AAC fixtures.
    #[test]
    fn native_linux_recorded_audio_survives_reordered_video_export() {
        let _lock = NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = Evidence::new("recording-to-export");
        let first = directory.path().join("first.mp4");
        let second = directory.path().join("second.mp4");
        let source = directory.path().join("recording.mp4");
        let boundary = crate::linux_media::recording_audio_fixture(&first, &[440.0], 700);
        crate::linux_media::recording_audio_fixture(&second, &[880.0], 700);
        crate::linux_media::merge_segments(&[first.clone(), second.clone()], &source).unwrap();
        let original_hashes = [&first, &second, &source].map(|path| hash(path));
        for (name, segments, tones) in [
            (
                "reordered",
                vec![clip(boundary + 0.1, boundary + 0.4), clip(0.1, 0.4)],
                [880.0, 440.0],
            ),
            (
                "zero-start",
                vec![clip(0.0, 0.3), clip(boundary + 0.1, boundary + 0.4)],
                [440.0, 880.0],
            ),
            (
                "pause-boundary",
                vec![clip(boundary - 0.3, boundary + 0.3)],
                [440.0, 880.0],
            ),
        ] {
            let output = directory.path().join(format!("{name}.mp4"));
            let (_, _, duration) = export(&source, &output, &segments);
            assert!((duration - 0.6).abs() < 0.001, "video duration {duration}");
            let frames = decode(&output, false);
            assert!(!frames.is_empty());
            for (_, pixels) in &frames {
                assert_color([pixels[0], pixels[1], pixels[2]], [0, 255, 0]);
            }
            let audio = decode(&output, true);
            let audio_seconds = audio_duration(&audio);
            assert!((audio_seconds - duration).abs() <= 1.0 / 48_000.0 + 1e-6);
            for (start, end, expected) in [(0.05, 0.25, tones[0]), (0.35, 0.55, tones[1])] {
                let samples: Vec<_> = audio
                    .iter()
                    .flat_map(|(pts, bytes)| {
                        bytes.as_chunks::<4>().0.iter().enumerate().filter_map(
                            move |(index, pair)| {
                                let time = pts + index as f64 / 48_000.0;
                                (time >= start && time < end)
                                    .then(|| i16::from_le_bytes([pair[0], pair[1]]))
                            },
                        )
                    })
                    .collect();
                assert!(samples.len() > 9000);
                let amplitude = |frequency: f64| {
                    let (sin, cos) = samples.iter().enumerate().fold(
                        (0.0, 0.0),
                        |(sin, cos), (index, value)| {
                            let phase = std::f64::consts::TAU * frequency * index as f64 / 48_000.0;
                            let value = f64::from(*value) / 32768.0;
                            (sin + value * phase.sin(), cos + value * phase.cos())
                        },
                    );
                    2.0 * sin.hypot(cos) / samples.len() as f64
                };
                let other = if expected == 440.0 { 880.0 } else { 440.0 };
                assert!(amplitude(expected) > 0.08, "{name}: missing {expected} Hz");
                assert!(amplitude(other) < 0.01, "{name}: unexpected {other} Hz");
            }
            assert_eq!(
                [&first, &second, &source].map(|path| hash(path)),
                original_hashes
            );
            std::fs::write(directory.path().join(format!("{name}.txt")), format!(
                "recording_encoder=avenc_aac\nmerged_segment_boundary={boundary}\noutput_video_duration={duration}\noutput_audio_duration={audio_seconds}\nfirst_tone={}\nsecond_tone={}\nsource_sha256={}\noutput_sha256={}\n",
                tones[0], tones[1], hash(&source), hash(&output),
            )).unwrap();
        }
    }

    #[test]
    fn native_linux_export_non_keyframe_cuts_reorder_and_preserve_audio() {
        let _lock = NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = Evidence::new("reorder-audio");
        let source = directory.path().join("source.mp4");
        fixture(&source, true, false);
        let before = hash(&source);
        let output = directory.path().join("edited.mp4");
        let (width, height, duration) =
            export(&source, &output, &[clip(2.35, 2.85), clip(0.35, 1.15)]);
        assert_eq!((width, height), (64, 48));
        assert!((duration - 1.3).abs() < 0.001, "{duration}");
        let video = decode(&output, false);
        assert!(video.len() >= 12);
        assert_color(color_at(&video, 0.0), [0, 0, 255]);
        assert_color(color_at(&video, 0.45), [0, 0, 255]);
        assert_color(color_at(&video, 0.55), [255, 0, 0]);
        assert_color(color_at(&video, 1.25), [0, 255, 0]);
        let audio = decode(&output, true);
        let first = frequency(&audio, 0.1, 0.4);
        let second = frequency(&audio, 0.6, 1.05);
        let audio_transition = transition_time(&audio, 0.5);
        assert!(
            (audio_transition - 0.5).abs() < 0.025,
            "audio/video boundary drift: {audio_transition}"
        );
        assert!((audio_duration(&audio) - 1.3).abs() < 0.00005);
        assert!((first - 660.0).abs() < 15.0, "first clip audio {first}");
        assert!((second - 220.0).abs() < 15.0, "reordered audio {second}");
        assert_eq!(hash(&source), before);
        std::fs::write(directory.path().join("assertions.json"), serde_json::json!({
            "sourceSha256":before,"outputSha256":hash(&output),"duration":duration,
            "dimensions":[width,height],"decodedFrames":video.len(),"audioFrequencies":[first,second],"audioTransition":audio_transition,"audioDuration":audio_duration(&audio),
            "sourceUnchanged":true,"requestedDuration":1.3,"segments":[[2.35,2.85],[0.35,1.15]]
        }).to_string()).unwrap();
    }

    #[test]
    fn native_linux_export_static_variable_frames_and_silent_middle_deletion() {
        let _lock = NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = Evidence::new("variable-silent");
        let source = directory.path().join("source.mp4");
        fixture(&source, false, true);
        let before = hash(&source);
        let output = directory.path().join("edited.mp4");
        let (_, _, duration) = export(&source, &output, &[clip(0.35, 1.15), clip(1.55, 2.85)]);
        assert!((duration - 2.1).abs() < 0.01, "{duration}");
        assert!(!inspect(&output).unwrap().audio);
        let frames = decode(&output, false);
        assert_color(color_at(&frames, 0.0), [0, 255, 0]);
        assert_color(color_at(&frames, 0.75), [0, 255, 0]);
        assert_color(color_at(&frames, 0.85), [255, 255, 255]);
        assert_color(color_at(&frames, 2.05), [255, 255, 255]);
        assert_eq!(before, hash(&source));
        std::fs::write(directory.path().join("assertions.json"), serde_json::json!({
            "sourceSha256":before,"outputSha256":hash(&output),"duration":duration,"audio":false,
            "decodedFrames":frames.len(),"sourceUnchanged":true,"requestedDuration":2.1
        }).to_string()).unwrap();
    }

    #[test]
    fn native_linux_export_rotation_presets_and_input_validation() {
        let _lock = NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = Evidence::new("rotation-validation");
        let source = directory.path().join("rotated.mp4");
        fixture(&source, false, false);
        let mut bytes = std::fs::read(&source).unwrap();
        let tkhd = bytes.windows(4).position(|part| part == b"tkhd").unwrap();
        let payload = tkhd + 4;
        let matrix = payload + if bytes[payload] == 1 { 52 } else { 40 };
        for (index, value) in [0i32, 65536, 0, -65536, 0, 0, 0, 0, 1 << 30]
            .into_iter()
            .enumerate()
        {
            bytes[matrix + index * 4..matrix + index * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
        std::fs::write(&source, &bytes).unwrap();
        let output = directory.path().join("edited.mp4");
        assert_eq!(export(&source, &output, &[clip(0.35, 1.15)]).0, 48);
        assert_eq!(inspect(&output).unwrap().height, 64);
        assert_color(color_at(&decode(&output, false), 0.0), [255, 0, 0]);
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        for segments in [
            vec![clip(0.0, 0.0)],
            vec![clip(-1.0, 1.0)],
            vec![clip(0.0, 9.0)],
            vec![clip(0.0, 2.0), clip(1.0, 3.0)],
            vec![clip(f64::NAN, 1.0)],
        ] {
            assert!(export_video_with_annotations_controlled(
                &source,
                &segments,
                &[],
                &[],
                VideoExportPreset::Original,
                &ExportControl::default()
            )
            .is_err());
        }
        let metadata = Source {
            width: 1920,
            height: 1080,
            duration: 1.0,
            audio: false,
        };
        assert_eq!(dimensions(&metadata, VideoExportPreset::Share), (1080, 606));
        assert_eq!(dimensions(&metadata, VideoExportPreset::Small), (720, 404));
    }

    #[test]
    fn native_linux_export_raw_audio_splice_is_sample_aligned() {
        let _lock = NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = Evidence::new("raw-audio-splice");
        let source = directory.path().join("source.mp4");
        fixture(&source, true, false);
        let pipe =
            pipeline("appsrc name=audio format=time ! appsink name=sink sync=false").unwrap();
        let src = pipe.by_name("audio").unwrap().downcast::<AppSrc>().unwrap();
        src.set_caps(Some(
            &gst::Caps::builder("audio/x-raw")
                .field("format", "S16LE")
                .field("layout", "interleaved")
                .field("channels", 2i32)
                .field("channel-mask", gst::Bitmask::new(3))
                .field("rate", 48000i32)
                .build(),
        ));
        let sink = pipe.by_name("sink").unwrap().downcast::<AppSink>().unwrap();
        pipe.set_state(gst::State::Playing).unwrap();
        render_audio(
            &source,
            &[clip(2.35, 2.85), clip(0.35, 1.15)],
            &src,
            &ExportControl::default().rendering(),
            &AtomicBool::new(false),
            &bus(&pipe).unwrap(),
        )
        .unwrap();
        let mut samples = Vec::new();
        while let Some(sample) = sink.try_pull_sample(gst::ClockTime::from_seconds(2)) {
            samples.push((
                sample_time(&sample).unwrap() as f64 / 1e9,
                sample.buffer().unwrap().map_readable().unwrap().to_vec(),
            ));
        }
        assert!(sink.is_eos());
        let transition = transition_time(&samples, 0.5);
        eprintln!(
            "raw audio splice at {transition}, length {}",
            audio_duration(&samples)
        );
        assert!((transition - 0.5).abs() < 0.02);
        assert!((audio_duration(&samples) - 1.3).abs() < 0.00003);
        let output = directory.path().join("edited.mp4");
        export(&source, &output, &[clip(2.35, 2.85), clip(0.35, 1.15)]);
        let encoded = decode(&output, true);
        fn pcm(samples: &[(f64, Vec<u8>)]) -> Vec<f64> {
            let mut output = vec![0.0; 70_000];
            for (time, bytes) in samples {
                let start = (time * 48_000.0).round() as usize;
                for (offset, frame) in bytes.as_chunks::<4>().0.iter().enumerate() {
                    if let Some(value) = output.get_mut(start + offset) {
                        *value = f64::from(i16::from_le_bytes([frame[0], frame[1]]));
                    }
                }
            }
            output
        }
        let raw = pcm(&samples);
        let encoded = pcm(&encoded);
        let lag = (-1024i32..=1024)
            .min_by(|a, b| {
                let error = |lag: i32| {
                    (20_000..28_000)
                        .map(|index| {
                            let delta = raw[index] - encoded[(index as i32 + lag) as usize];
                            delta * delta
                        })
                        .sum::<f64>()
                };
                error(*a).total_cmp(&error(*b))
            })
            .unwrap();
        eprintln!("encoded audio lag versus raw cut: {lag} samples");
        assert!(
            lag.abs() <= 96,
            "AAC export shifted the splice by {lag} samples"
        );
        std::fs::write(directory.path().join("assertions.json"),serde_json::json!({
            "aacLagSamples":lag,"sampleRate":48000,"requestedDuration":1.3,"rawDuration":audio_duration(&samples)
        }).to_string()).unwrap();
    }

    #[test]
    fn native_linux_export_preserves_audio_gaps_and_short_track() {
        let _lock = NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = Evidence::new("audio-gaps");
        let source = directory.path().join("source.mp4");
        fixture_audio_range(&source, Some(40..350), false);
        let output = directory.path().join("edited.mp4");
        let before = hash(&source);
        let (_, _, duration) = export(&source, &output, &[clip(0.1, 0.8), clip(3.7, 4.0)]);
        let audio = decode(&output, true);
        assert!((duration - 1.0).abs() < 0.05);
        assert!((frequency(&audio, 0.4, 0.6) - 220.0).abs() < 15.0);
        for (from, to) in [(0.0, 0.25), (0.8, 0.95)] {
            let loudest =
                audio
                    .iter()
                    .flat_map(|(pts, bytes)| {
                        bytes.as_chunks::<4>().0.iter().enumerate().filter_map(
                            move |(index, pair)| {
                                let time = pts + index as f64 / 48_000.0;
                                (time >= from && time < to)
                                    .then(|| i16::from_le_bytes([pair[0], pair[1]]).unsigned_abs())
                            },
                        )
                    })
                    .max()
                    .unwrap();
            assert!(
                loudest < 500,
                "source audio gap was shifted or replaced with sound: {loudest}"
            );
        }
        assert_eq!(before, hash(&source));
        assert_eq!(decode(&output, false).len(), 10);
    }

    #[test]
    fn native_linux_export_size_preset_decodes_every_output_frame() {
        let _lock = NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = Evidence::new("size-preset");
        let source = directory.path().join("source.mp4");
        let pipe = pipeline("videotestsrc num-buffers=6 pattern=ball ! video/x-raw,width=1280,height=720,framerate=10/1 ! x264enc tune=zerolatency ! video/x-h264,stream-format=avc,alignment=au ! mp4mux ! filesink name=output").unwrap();
        location(&pipe, "output", &source).unwrap();
        pipe.set_state(gst::State::Playing).unwrap();
        finish(&pipe);
        drop(pipe);
        let before = hash(&source);
        let (file, width, height, duration) = export_video_with_annotations_controlled(
            &source,
            &[clip(0.05, 0.55)],
            &[],
            &[],
            VideoExportPreset::Small,
            &ExportControl::default(),
        )
        .unwrap();
        let output = directory.path().join("edited.mp4");
        std::fs::rename(file, &output).unwrap();
        assert_eq!((width, height), (720, 404));
        assert!((duration - 0.5).abs() < 0.01);
        let frames = decode(&output, false);
        assert_eq!(frames.len(), 6);
        assert!(frames.iter().all(|(_, bytes)| bytes.len() == 720 * 404 * 4));
        assert_eq!(before, hash(&source));
    }

    #[test]
    fn native_linux_export_rejects_anamorphic_video_without_partial_output() {
        let _lock = NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = Evidence::new("anamorphic-rejection");
        let source = directory.path().join("source.mp4");
        let pipe = pipeline("videotestsrc num-buffers=6 ! video/x-raw,width=64,height=48,framerate=10/1,pixel-aspect-ratio=2/1 ! x264enc tune=zerolatency ! video/x-h264,stream-format=avc,alignment=au ! mp4mux ! filesink name=output").unwrap();
        location(&pipe, "output", &source).unwrap();
        pipe.set_state(gst::State::Playing).unwrap();
        finish(&pipe);
        drop(pipe);
        let before = hash(&source);
        let staged = Arc::new(Mutex::new(None));
        let seen = staged.clone();
        let control = ExportControl::new(move |event| {
            if event.phase == ExportPhase::Rendering && event.progress == Some(0.0) {
                *seen.lock().unwrap() = empty_staging_directory();
            }
        });
        let error = export_video_with_annotations_controlled(
            &source,
            &[clip(0.0, 0.5)],
            &[],
            &[],
            VideoExportPreset::Original,
            &control,
        )
        .unwrap_err();
        assert!(error.to_string().contains("square-pixel"));
        assert_eq!(hash(&source), before);
        assert!(!staged
            .lock()
            .unwrap()
            .as_ref()
            .expect("an export owns staging before inspection")
            .exists());
    }

    fn empty_staging_directory() -> Option<PathBuf> {
        std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("kiri-video-export-")
            })
            .map(|entry| entry.path())
            .find(|path| std::fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_none()))
    }

    #[test]
    fn linux_export_finalization_preserves_an_already_queued_eos() {
        gst::init().unwrap();
        let bus = gst::Bus::new();
        bus.post(gst::message::Eos::builder().build()).unwrap();
        let started = Instant::now();
        wait_for_output(&bus, &ExportControl::default().rendering()).unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        let cancelled = ExportControl::default();
        cancelled.request_cancel();
        assert_eq!(
            wait_for_output(&bus, &cancelled.rendering())
                .unwrap_err()
                .to_string(),
            EXPORT_CANCELLED
        );
    }

    #[test]
    fn linux_export_rejects_unsupported_content_before_media_work() {
        let range = ExportControl::default().rendering();
        let path = Path::new("missing.mp4");
        for segments in [
            vec![VideoSegment {
                speed: 2.0,
                ..clip(0.0, 1.0)
            }],
            vec![VideoSegment {
                speed: 0.5,
                ..clip(0.0, 1.0)
            }],
        ] {
            assert!(platform_export(
                path,
                path,
                &segments,
                &[],
                &[],
                VideoExportPreset::Original,
                &range
            )
            .unwrap_err()
            .to_string()
            .contains("normal-speed cuts only"));
        }
        let effect = VideoEffect {
            kind: VideoEffectKind::Mask,
            start: 0.0,
            end: 1.0,
            ..Default::default()
        };
        assert!(platform_export(
            path,
            path,
            &[clip(0.0, 1.0)],
            &[effect],
            &[],
            VideoExportPreset::Original,
            &range
        )
        .unwrap_err()
        .to_string()
        .contains("privacy masks"));
        let annotation = PreparedVideoAnnotation {
            start: 0.0,
            end: 1.0,
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            kind: super::super::VideoAnnotationKind::Overlay,
            image: image::RgbaImage::new(1, 1),
            amount: 1.0,
            layer: 0,
        };
        assert!(platform_export(
            path,
            path,
            &[clip(0.0, 1.0)],
            &[],
            &[annotation],
            VideoExportPreset::Original,
            &range
        )
        .unwrap_err()
        .to_string()
        .contains("annotations"));
    }

    #[test]
    fn native_linux_export_cancel_cleans_partial_files_and_preserves_source() {
        let _lock = NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = Evidence::new("cancel-cleanup");
        let source = directory.path().join("source.mp4");
        fixture(&source, true, false);
        let before = hash(&source);
        let slot: Arc<OnceLock<std::sync::Weak<super::super::ExportControlInner>>> =
            Arc::new(OnceLock::new());
        let observed = slot.clone();
        let events = Arc::new(Mutex::new(Vec::new()));
        let seen = events.clone();
        let staging = Arc::new(Mutex::new(None::<PathBuf>));
        let staged = staging.clone();
        let control = ExportControl::new(move |event| {
            seen.lock().unwrap().push(event.progress);
            if event.phase == ExportPhase::Rendering && event.progress == Some(0.0) {
                // Native export tests are serialized; no other native test
                // uses this wrapper's staging prefix on Linux.
                *staged.lock().unwrap() = empty_staging_directory();
                // Move beyond the production progress throttle without slowing
                // the encoder or relying on machine performance for cancellation.
                std::thread::sleep(Duration::from_millis(120));
            }
            if event.phase == ExportPhase::Rendering
                && event
                    .progress
                    .is_some_and(|value| value > 0.0 && value < 1.0)
            {
                if let Some(inner) = observed.get().and_then(std::sync::Weak::upgrade) {
                    ExportControl(inner).request_cancel();
                }
            }
        });
        slot.set(Arc::downgrade(&control.0)).unwrap();
        let start = Instant::now();
        let result = export_video_with_annotations_controlled(
            &source,
            &[clip(0.0, 3.0)],
            &[],
            &[] as &[VideoAnnotation],
            VideoExportPreset::Original,
            &control,
        );
        assert_eq!(result.unwrap_err().to_string(), EXPORT_CANCELLED);
        assert!(start.elapsed() < Duration::from_secs(10));
        assert!(events
            .lock()
            .unwrap()
            .iter()
            .flatten()
            .all(|progress| *progress < 1.0));
        assert_eq!(before, hash(&source));
        let staging = staging
            .lock()
            .unwrap()
            .clone()
            .expect("the cancelled export must create owned staging");
        assert!(
            !staging.exists(),
            "cancelled export left its staging directory"
        );
    }
}
