//! Command line of the benchmark

use std::{fmt, time::Duration};

use crate::light::LIGHT_RADII;

/// What the benchmark does to the map while it measures
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    /// No edits, the steady state after loading
    Idle,
    /// Zooms through the zoom levels and resizes the window
    Zoom,
    /// Edits this many tiles spread over the map every frame, so that the scene is updated
    Edits(usize),
    /// Changes every layer of every tile every frame, so that everything is rebuilt
    Churn,
    /// Moves the view every frame and changes the zoom every few seconds, so that every frame
    /// shows new pixels
    Pan,
    /// Moves the cursor of the light place tool by a tile every frame, so that the scene is lit
    /// with the preview of a light of this radius at the new tile
    LightPreview(u8),
}

impl Scenario {
    /// Names and descriptions for `--list`
    pub const DESCRIPTIONS: [(&'static str, &'static str); 6] = [
        ("idle", "No edits, the steady state after loading the map"),
        (
            "zoom",
            "Steps through the zoom levels and window sizes, every 2 seconds",
        ),
        (
            "edits=N",
            "Every frame, edits N tiles spread over the map (incremental updates)",
        ),
        (
            "churn",
            "Every frame, changes ground, objects, structures, walls, roofs, shadows, soldiers, \
             lights and items of every tile (full rebuilds)",
        ),
        (
            "pan",
            "Every frame, moves the view on a loop over the map, every 5 seconds the next zoom \
             of 0.5, 1, 2 and 4",
        ),
        (
            "light-preview=R",
            "Every frame, moves the cursor of the light place tool to the next tile, previewing \
             a light of radius R (1 to 8)",
        ),
    ];

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "idle" => Ok(Scenario::Idle),
            "zoom" => Ok(Scenario::Zoom),
            "churn" => Ok(Scenario::Churn),
            "pan" => Ok(Scenario::Pan),
            _ => {
                if let Some(count) = value.strip_prefix("edits=") {
                    match count.parse() {
                        Ok(count) if count > 0 => Ok(Scenario::Edits(count)),
                        _ => Err(format!("invalid number of edits in `{}`", value)),
                    }
                } else if let Some(radius) = value.strip_prefix("light-preview=") {
                    match radius.parse() {
                        Ok(radius) if LIGHT_RADII.contains(&radius) => {
                            Ok(Scenario::LightPreview(radius))
                        }
                        _ => Err(format!("invalid light radius in `{}`", value)),
                    }
                } else {
                    Err(format!("unknown scenario `{}`", value))
                }
            }
        }
    }
}

impl fmt::Display for Scenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scenario::Idle => write!(f, "idle"),
            Scenario::Zoom => write!(f, "zoom"),
            Scenario::Edits(count) => write!(f, "edits={}", count),
            Scenario::Churn => write!(f, "churn"),
            Scenario::Pan => write!(f, "pan"),
            Scenario::LightPreview(radius) => write!(f, "light-preview={}", radius),
        }
    }
}

/// How long each scenario runs
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Duration(Duration),
    Frames(u64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Args {
    /// Run one after another, each with a fresh map
    pub scenarios: Vec<Scenario>,
    pub length: Length,
    /// Samples within this time after the start of a scenario are not part of the means
    pub warmup: Duration,
    /// Time between samples
    pub interval: Duration,
    /// Window size in physical pixels, which the textures are sized for
    pub width: u32,
    pub height: u32,
    /// Presents without waiting for the vertical blank, to measure frame times above 60 FPS
    pub no_vsync: bool,
    /// Overrides the scale factor of the window
    pub scale: Option<f32>,
    /// Also writes the JSON lines to this file
    pub out: Option<String>,
    /// Compares the rendered map with the CPU replay after the warmup of each scenario
    pub verify: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            scenarios: vec![Scenario::Idle],
            length: Length::Duration(Duration::from_secs(60)),
            warmup: Duration::from_secs(2),
            interval: Duration::from_secs(1),
            width: 1920,
            height: 1080,
            no_vsync: false,
            scale: None,
            out: None,
            verify: false,
        }
    }
}

/// What the command line asks for
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Run(Args),
    List,
    Help,
}

pub const USAGE: &str = "\
Usage: bench [options]

Runs the map viewer natively on a synthetic map and prints one JSON object per line to stdout
(`start`, `sample`, `summary`, `notice`). Logs go to stderr.

Options:
  --list                 Print the scenarios and exit
  --scenario LIST        Comma separated scenarios to run one after another, can be repeated
                         (default: idle)
  --duration SECS        Run each scenario for this long (default: 60)
  --frames N             Run each scenario for N frames instead of a duration
  --warmup SECS          Exclude samples of the first SECS from the means (default: 2)
  --interval SECS        Time between samples (default: 1)
  --width W              Window width in physical pixels (default: 1920)
  --height H             Window height in physical pixels (default: 1080)
  --scale F              Override the window scale factor, e.g. 2 for HiDPI
  --no-vsync             Don't wait for the vertical blank, frame times can go below 16.7 ms
  --out PATH             Also write the JSON lines to PATH
  --verify               After the warmup of each scenario, step the view over the map at zoom 1
                         and compare every pixel with the CPU replay of the draw list
  --help                 Print this help
";

fn seconds(name: &str, value: &str) -> Result<Duration, String> {
    value
        .parse::<f64>()
        .ok()
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.)
        .map(Duration::from_secs_f64)
        .ok_or_else(|| format!("invalid number of seconds for {}: `{}`", name, value))
}

fn number<T: std::str::FromStr>(name: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {}: `{}`", name, value))
}

/// Parses the arguments without the program name
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut parsed = Args::default();
    let mut scenarios = vec![];
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        // Both `--name value` and `--name=value`
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name.to_owned(), Some(value)),
            _ => (arg.clone(), None),
        };
        let mut value = || -> Result<String, String> {
            match inline {
                Some(value) => Ok(value.to_owned()),
                None => args
                    .next()
                    .ok_or_else(|| format!("missing value for {}", name)),
            }
        };
        match name.as_str() {
            "--list" => return Ok(Command::List),
            "--no-vsync" => parsed.no_vsync = true,
            "--verify" => parsed.verify = true,
            "--help" | "-h" => return Ok(Command::Help),
            "--scenario" => {
                for scenario in value()?.split(',').filter(|s| !s.is_empty()) {
                    scenarios.push(Scenario::parse(scenario)?);
                }
            }
            "--duration" => parsed.length = Length::Duration(seconds(&name, &value()?)?),
            "--frames" => match number(&name, &value()?)? {
                0 => return Err("--frames must be at least 1".to_owned()),
                frames => parsed.length = Length::Frames(frames),
            },
            "--warmup" => parsed.warmup = seconds(&name, &value()?)?,
            "--interval" => match seconds(&name, &value()?)? {
                interval if interval.is_zero() => {
                    return Err("--interval must be more than 0".to_owned())
                }
                interval => parsed.interval = interval,
            },
            "--width" => parsed.width = number(&name, &value()?)?,
            "--height" => parsed.height = number(&name, &value()?)?,
            "--scale" => match number::<f32>(&name, &value()?)? {
                scale if scale > 0. && scale.is_finite() => parsed.scale = Some(scale),
                _ => return Err("--scale must be more than 0".to_owned()),
            },
            "--out" => parsed.out = Some(value()?),
            _ => return Err(format!("unknown argument `{}`", arg)),
        }
    }
    if parsed.width == 0 || parsed.height == 0 {
        return Err("the window size must be at least 1×1".to_owned());
    }
    if !scenarios.is_empty() {
        parsed.scenarios = scenarios;
    }
    Ok(Command::Run(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Result<Args, String> {
        match parse(args.iter().map(|arg| arg.to_string()))? {
            Command::Run(args) => Ok(args),
            command => panic!("not a run: {:?}", command),
        }
    }

    #[test]
    fn defaults() {
        assert_eq!(run(&[]), Ok(Args::default()));
    }

    #[test]
    fn scenarios_are_comma_separated_and_repeatable() {
        let args = run(&["--scenario", "idle,edits=8", "--scenario=churn,pan"]).unwrap();
        assert_eq!(
            args.scenarios,
            vec![
                Scenario::Idle,
                Scenario::Edits(8),
                Scenario::Churn,
                Scenario::Pan
            ]
        );
    }

    #[test]
    fn options() {
        let args = run(&[
            "--duration",
            "1.5",
            "--warmup=0",
            "--interval",
            "0.25",
            "--width",
            "800",
            "--height=600",
            "--scale",
            "2",
            "--out",
            "/tmp/bench.jsonl",
            "--no-vsync",
            "--verify",
        ])
        .unwrap();
        assert_eq!(args.length, Length::Duration(Duration::from_millis(1500)));
        assert_eq!(args.warmup, Duration::ZERO);
        assert_eq!(args.interval, Duration::from_millis(250));
        assert_eq!((args.width, args.height), (800, 600));
        assert_eq!(args.scale, Some(2.));
        assert_eq!(args.out.as_deref(), Some("/tmp/bench.jsonl"));
        assert!(args.no_vsync);
        assert!(args.verify);
        assert_eq!(
            run(&["--frames", "120"]).unwrap().length,
            Length::Frames(120)
        );
    }

    #[test]
    fn list_and_help() {
        let parse = |args: &[&str]| parse(args.iter().map(|arg| arg.to_string()));
        assert_eq!(parse(&["--list"]), Ok(Command::List));
        assert_eq!(parse(&["--scenario", "idle", "--help"]), Ok(Command::Help));
    }

    #[test]
    fn invalid_arguments() {
        for args in [
            &["--scenario", "fast"][..],
            &["--scenario", "edits=0"],
            &["--scenario", "edits=x"],
            &["--scenario", "light-preview=0"],
            &["--scenario", "light-preview=9"],
            &["--scenario", "light-preview"],
            &["--duration", "-1"],
            &["--duration"],
            &["--frames", "0"],
            &["--interval", "0"],
            &["--width", "0"],
            &["--scale", "0"],
            &["--verbose"],
        ] {
            assert!(run(args).is_err(), "{:?}", args);
        }
    }

    #[test]
    fn scenario_names_round_trip() {
        for scenario in [
            Scenario::Idle,
            Scenario::Zoom,
            Scenario::Edits(3),
            Scenario::Churn,
            Scenario::Pan,
            Scenario::LightPreview(8),
        ] {
            assert_eq!(Scenario::parse(&scenario.to_string()), Ok(scenario));
        }
    }
}
