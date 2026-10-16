//! Measuring the scenarios and writing the JSON lines
//!
//! Every frame the GPU memory is read from the allocator of wgpu, so that short peaks like a
//! full rebuild of the scene are not missed. The GPU time of a frame is the sum of the render pass
//! timestamps of `RenderDiagnosticsPlugin`, where the adapter has timestamp queries. Samples are
//! written every `--interval`, a summary at the end of each scenario.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Write},
};

use bevy::{
    diagnostic::DiagnosticsStore,
    platform::time::Instant,
    prelude::*,
    render::{
        renderer::{RenderAdapterInfo, RenderDevice},
        settings::WgpuFeatures,
    },
    window::PrimaryWindow,
};
use serde_json::{json, Value};

use super::{args::Length, Bench};
use crate::{render::ReplayScene, systems::pixel_view::PixelView};

/// Number of allocations listed in the summary
const LARGEST_ALLOCATIONS: usize = 10;
/// Number of groups of allocations listed in the summary
const ALLOCATION_GROUPS: usize = 15;

/// Writes the JSON lines and measures the current scenario
#[derive(Resource)]
pub struct Report {
    file: Option<File>,
    run: Option<Run>,
    /// The allocator report is missing, which was already reported
    no_allocator_report: bool,
}

/// GPU memory in bytes
#[derive(Clone, Copy, Default)]
struct Memory {
    allocated: u64,
    reserved: u64,
}

impl Memory {
    fn max(self, other: Memory) -> Memory {
        Memory {
            allocated: self.allocated.max(other.allocated),
            reserved: self.reserved.max(other.reserved),
        }
    }
}

/// What happened since the last sample, or since the start of the scenario
#[derive(Default)]
struct Counters {
    frames: u64,
    frame_ms_sum: f64,
    frame_ms_max: f64,
    /// Frames with GPU timestamps and their sum
    gpu_frames: u64,
    gpu_ms_sum: f64,
    peak: Option<Memory>,
    builds_full: u64,
    builds_incremental: u64,
}

impl Counters {
    fn json(&self) -> Value {
        json!({
            "builds_full": self.builds_full,
            "builds_incremental": self.builds_incremental,
        })
    }
}

/// The measurements of the current scenario
struct Run {
    scenario: String,
    /// Real time at the start in seconds
    start: f64,
    /// Real time of the last frame in seconds
    now: f64,
    frames: u64,
    last_sample: f64,
    interval: Counters,
    total: Counters,
    /// Frame times after the warmup
    frame_ms: Vec<f64>,
    /// GPU times of the frames after the warmup
    gpu_ms: Vec<f64>,
    /// Time of the newest GPU timestamps that were counted
    last_gpu_time: Instant,
    /// The scenario is paused (`--verify`), the time doesn't count
    paused: bool,
    /// Result of `--verify`
    verification: Option<Value>,
    peak_after_warmup: Option<Memory>,
    /// Sum and number of the GPU memory of the frames after the warmup
    memory_sum: (u128, u128, u64),
    last_memory: Option<Memory>,
    /// The allocations at the peak of the allocated memory, by name: number and bytes
    allocations_at_peak: Vec<(String, u64)>,
    /// Version of the draws of the scene that were counted
    draws_version: Option<u64>,
}

impl Report {
    pub fn new(out: Option<&str>) -> io::Result<Self> {
        Ok(Self {
            file: out.map(File::create).transpose()?,
            run: None,
            no_allocator_report: false,
        })
    }

    /// Frames since the start of the current scenario, `None` between scenarios
    pub fn frame(&self) -> Option<u64> {
        self.run.as_ref().map(|run| run.frames)
    }

    /// Pauses or resumes the current scenario. While it is paused, its time stands still and
    /// only the peak memory is measured.
    pub fn set_paused(&mut self, paused: bool) {
        if let Some(run) = &mut self.run {
            run.paused = paused;
        }
    }

    /// Adds the result of `--verify` to the summary of the current scenario
    pub fn set_verification(&mut self, verification: Value) {
        if let Some(run) = &mut self.run {
            run.verification = Some(verification);
        }
    }

    /// Seconds since the start of the current scenario
    pub fn elapsed(&self) -> f32 {
        self.run
            .as_ref()
            .map_or(0., |run| (run.now - run.start) as f32)
    }

    fn write(&mut self, value: Value) {
        let line = value.to_string();
        println!("{}", line);
        if let Some(file) = &mut self.file {
            if let Err(error) = writeln!(file, "{}", line) {
                error!("Error writing the output file: {}", error);
                self.file = None;
            }
        }
    }
}

/// Resident memory of the process in bytes, `None` where `/proc` doesn't have it
fn resident_memory() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}

fn mib(bytes: u64) -> f64 {
    (bytes as f64 / (1024. * 1024.) * 10.).round() / 10.
}

fn memory_json(memory: Option<Memory>) -> Value {
    match memory {
        Some(memory) => json!({
            "allocated": memory.allocated,
            "reserved": memory.reserved,
            "allocated_mib": mib(memory.allocated),
            "reserved_mib": mib(memory.reserved),
        }),
        None => Value::Null,
    }
}

fn round(value: f64) -> f64 {
    (value * 100.).round() / 100.
}

/// GPU time of the frames whose render diagnostics arrived after `after`, in milliseconds
///
/// The diagnostics of a frame all have the same time. Passes of all views are summed up.
fn gpu_frames(store: &DiagnosticsStore, after: Instant) -> BTreeMap<Instant, f64> {
    let mut frames = BTreeMap::new();
    for diagnostic in store.iter() {
        if !diagnostic.path().as_str().ends_with("/elapsed_gpu") {
            continue;
        }
        for measurement in diagnostic.measurements() {
            if measurement.time > after {
                *frames.entry(measurement.time).or_default() += measurement.value;
            }
        }
    }
    frames
}

pub fn start_scenario(
    time: Res<Time<Real>>,
    bench: Res<Bench>,
    mut report: ResMut<Report>,
    adapter: Option<Res<RenderAdapterInfo>>,
    device: Res<RenderDevice>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let now = time.elapsed_secs_f64();
    let timestamps = WgpuFeatures::TIMESTAMP_QUERY | WgpuFeatures::TIMESTAMP_QUERY_INSIDE_PASSES;
    let scenario = bench.scenario().to_string();
    let length = match bench.args.length {
        Length::Duration(duration) => json!({ "seconds": duration.as_secs_f64() }),
        Length::Frames(frames) => json!({ "frames": frames }),
    };
    let physical = window.resolution.physical_size();
    report.write(json!({
        "type": "start",
        "scenario": scenario,
        "index": bench.current,
        "scenarios": bench.args.scenarios.len(),
        "length": length,
        "warmup_seconds": bench.args.warmup.as_secs_f64(),
        "window": {
            "physical_width": physical.x,
            "physical_height": physical.y,
            "scale_factor": window.resolution.scale_factor(),
        },
        "vsync": !bench.args.no_vsync,
        "verify": bench.args.verify,
        "gpu_timestamps": device.features().contains(timestamps),
        "adapter": adapter.as_ref().map(|adapter| adapter.name.clone()),
        "backend": adapter.as_ref().map(|adapter| format!("{:?}", adapter.backend)),
    }));
    report.run = Some(Run {
        scenario,
        start: now,
        now,
        frames: 0,
        last_sample: now,
        interval: Counters::default(),
        total: Counters::default(),
        frame_ms: vec![],
        gpu_ms: vec![],
        last_gpu_time: Instant::now(),
        paused: false,
        verification: None,
        peak_after_warmup: None,
        memory_sum: (0, 0, 0),
        last_memory: None,
        allocations_at_peak: vec![],
        draws_version: None,
    });
}

/// Measures the frame, writes a sample every interval and the summary at the end
pub fn measure(
    time: Res<Time<Real>>,
    mut bench: ResMut<Bench>,
    mut report: ResMut<Report>,
    device: Res<RenderDevice>,
    adapter: Option<Res<RenderAdapterInfo>>,
    scene: Res<ReplayScene>,
    pixel_view: Res<PixelView>,
    images: Res<Assets<Image>>,
    window: Single<&Window, With<PrimaryWindow>>,
    diagnostics: Res<DiagnosticsStore>,
) {
    let allocator_report = device.wgpu_device().generate_allocator_report();
    if allocator_report.is_none() && !report.no_allocator_report {
        report.no_allocator_report = true;
        let backend = adapter
            .as_ref()
            .map(|adapter| format!("{:?}", adapter.backend));
        // Only these backends of wgpu have an allocator report
        let expected = matches!(backend.as_deref(), Some("Vulkan" | "Dx12"));
        if expected {
            bench.failed = true;
        }
        report.write(json!({
            "type": "notice",
            "message": format!(
                "wgpu has no allocator report for the {} backend, the GPU memory is null",
                backend.as_deref().unwrap_or("unknown")
            ),
        }));
    }
    let memory = allocator_report.as_ref().map(|report| Memory {
        allocated: report.total_allocated_bytes,
        reserved: report.total_reserved_bytes,
    });

    let args = &bench.args;
    let warmup = args.warmup.as_secs_f64();
    let interval = args.interval.as_secs_f64();
    let length = args.length;
    let Some(run) = report.run.as_mut() else {
        return;
    };
    let now = time.elapsed_secs_f64();
    let gpu = gpu_frames(&diagnostics, run.last_gpu_time);
    if let Some(&last) = gpu.keys().next_back() {
        run.last_gpu_time = last;
    }
    if run.paused {
        // The paused time doesn't count, but peaks do
        let paused = now - run.now;
        run.start += paused;
        run.last_sample += paused;
        run.now = now;
        if let Some(memory) = memory {
            run.total.peak = Some(run.total.peak.map_or(memory, |peak| peak.max(memory)));
            run.interval.peak = Some(run.interval.peak.map_or(memory, |peak| peak.max(memory)));
        }
        return;
    }
    run.now = now;
    let elapsed = now - run.start;
    let after_warmup = elapsed >= warmup;
    // The first frame waited for the map to be built
    let frame_ms = if run.frames == 0 {
        None
    } else {
        Some(time.delta_secs_f64() * 1000.)
    };
    run.frames += 1;

    let previous_peak = run.total.peak;
    for counters in [&mut run.interval, &mut run.total] {
        if let Some(frame_ms) = frame_ms {
            counters.frames += 1;
            counters.frame_ms_sum += frame_ms;
            counters.frame_ms_max = counters.frame_ms_max.max(frame_ms);
        }
        if let Some(memory) = memory {
            counters.peak = Some(counters.peak.map_or(memory, |peak| peak.max(memory)));
        }
    }
    if let (Some(frame_ms), true) = (frame_ms, after_warmup) {
        run.frame_ms.push(frame_ms);
    }
    for &gpu_ms in gpu.values() {
        for counters in [&mut run.interval, &mut run.total] {
            counters.gpu_frames += 1;
            counters.gpu_ms_sum += gpu_ms;
        }
        if after_warmup {
            run.gpu_ms.push(gpu_ms);
        }
    }
    if let Some(memory) = memory {
        if after_warmup {
            run.peak_after_warmup = Some(run.peak_after_warmup.map_or(memory, |p| p.max(memory)));
            run.memory_sum.0 += u128::from(memory.allocated);
            run.memory_sum.1 += u128::from(memory.reserved);
            run.memory_sum.2 += 1;
        }
        let new_peak = previous_peak.is_none_or(|peak| memory.allocated > peak.allocated);
        if new_peak {
            if let Some(allocator_report) = &allocator_report {
                run.allocations_at_peak = allocator_report
                    .allocations
                    .iter()
                    .map(|allocation| (allocation.name.clone(), allocation.size))
                    .collect();
            }
        }
    }
    run.last_memory = memory;

    // The draws are packed again on every full build or update after edits
    if scene.is_changed() && run.draws_version != Some(scene.draws_version) {
        run.draws_version = Some(scene.draws_version);
        for counters in [&mut run.interval, &mut run.total] {
            if scene.full_build {
                counters.builds_full += 1;
            } else {
                counters.builds_incremental += 1;
            }
        }
    }

    let finished = match length {
        Length::Duration(duration) => elapsed >= duration.as_secs_f64(),
        Length::Frames(frames) => run.frames >= frames,
    };
    if now - run.last_sample < interval && !finished {
        return;
    }
    run.last_sample = now;
    let counters = std::mem::take(&mut run.interval);
    let shared = &scene.shared;
    let atlas_bytes = 4 * (shared.atlas.len() + shared.palettes.len() + shared.sprites.len());
    let texture = images
        .get(&pixel_view.image)
        .map(|image| image.size())
        .unwrap_or_default();
    let physical = window.resolution.physical_size();
    let sample = json!({
        "type": "sample",
        "scenario": run.scenario,
        "t": round(elapsed),
        "frame": run.frames,
        "warmup": !after_warmup,
        "gpu": memory_json(memory),
        "gpu_peak_interval": memory_json(counters.peak),
        "gpu_peak": memory_json(run.total.peak),
        "gpu_allocations": allocator_report.as_ref().map(|report| report.allocations.len()),
        "gpu_blocks": allocator_report.as_ref().map(|report| report.blocks.len()),
        "frame_ms_avg": (counters.frames > 0)
            .then(|| round(counters.frame_ms_sum / counters.frames as f64)),
        "frame_ms_max": (counters.frames > 0).then(|| round(counters.frame_ms_max)),
        "frames": counters.frames,
        "gpu_ms_avg": (counters.gpu_frames > 0)
            .then(|| round(counters.gpu_ms_sum / counters.gpu_frames as f64)),
        "window": [physical.x, physical.y],
        "pixel_view_texture": [texture.x, texture.y],
        "zoom": round(f64::from(pixel_view.zoom)),
        "scene": counters.json(),
        "atlas_bytes": atlas_bytes,
        "rss": resident_memory(),
    });
    let summary = finished.then(|| summary(run));
    report.write(sample);
    if let Some(summary) = summary {
        report.write(summary);
        report.run = None;
        bench.finished = true;
    }
}

/// A percentile of sorted values
fn percentile(sorted: &[f64], p: f64) -> Option<f64> {
    let last = sorted.len().checked_sub(1)?;
    Some(round(sorted[(last as f64 * p).round() as usize]))
}

fn summary(run: &mut Run) -> Value {
    let mut frame_ms = std::mem::take(&mut run.frame_ms);
    frame_ms.sort_by(f64::total_cmp);
    let mut gpu_ms = std::mem::take(&mut run.gpu_ms);
    gpu_ms.sort_by(f64::total_cmp);
    let (allocated_sum, reserved_sum, count) = run.memory_sum;
    let mean = (count > 0).then(|| Memory {
        allocated: (allocated_sum / u128::from(count)) as u64,
        reserved: (reserved_sum / u128::from(count)) as u64,
    });
    let mut largest = run.allocations_at_peak.clone();
    largest.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
    let largest: Vec<Value> = largest
        .iter()
        .take(LARGEST_ALLOCATIONS)
        .map(|(name, size)| json!({ "name": name, "bytes": size, "mib": mib(*size) }))
        .collect();
    // Unlabeled allocations all have the same name, so they are grouped by name and size in MiB
    let mut groups: BTreeMap<(String, u64), (u64, u64)> = BTreeMap::new();
    for (name, size) in &run.allocations_at_peak {
        let mib = (*size as f64 / (1024. * 1024.)).round() as u64;
        let group = groups.entry((name.clone(), mib)).or_default();
        group.0 += 1;
        group.1 += size;
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(_, (_, bytes))| std::cmp::Reverse(*bytes));
    let groups: Vec<Value> = groups
        .iter()
        .take(ALLOCATION_GROUPS)
        .map(|((name, mib_each), (count, bytes))| {
            json!({
                "name": name,
                "mib_each": mib_each,
                "count": count,
                "bytes": bytes,
                "mib": self::mib(*bytes),
            })
        })
        .collect();
    json!({
        "type": "summary",
        "scenario": run.scenario,
        "seconds": round(run.now - run.start),
        "frames": run.frames,
        "gpu_peak": memory_json(run.total.peak),
        "gpu_peak_after_warmup": memory_json(run.peak_after_warmup),
        "gpu_mean_after_warmup": memory_json(mean),
        "gpu_final": memory_json(run.last_memory),
        "largest_allocations_at_peak": largest,
        "allocation_groups_at_peak": groups,
        "frame_ms_p50": percentile(&frame_ms, 0.5),
        "frame_ms_p95": percentile(&frame_ms, 0.95),
        "frame_ms_max": frame_ms.last().map(|ms| round(*ms)),
        "gpu_ms_p50": percentile(&gpu_ms, 0.5),
        "gpu_ms_p95": percentile(&gpu_ms, 0.95),
        "gpu_ms_max": gpu_ms.last().map(|ms| round(*ms)),
        "gpu_frames": gpu_ms.len(),
        "verify": run.verification.take(),
        "scene": run.total.json(),
        "rss": resident_memory(),
    })
}
