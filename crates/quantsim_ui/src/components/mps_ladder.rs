use egui::Ui;
use egui_plot::{GridMark, HLine, Line, LineStyle, Plot, Points};
use std::sync::OnceLock;

const DATA: &str = include_str!("../../assets/benchmarks/mps-cap-curve.csv");
const ECHO_TOLERANCE: f64 = 2e-10;

#[derive(Debug)]
struct Sample {
    cap: usize,
    probability: f64,
    seconds: Option<f64>,
    peak_rss_bytes: u64,
}

fn samples() -> &'static [Sample] {
    static SAMPLES: OnceLock<Vec<Sample>> = OnceLock::new();
    SAMPLES.get_or_init(|| {
        DATA.lines()
            .skip(1)
            .map(|line| {
                let fields: [&str; 7] = line
                    .split(',')
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("expected seven columns in MPS benchmark CSV");
                Sample {
                    cap: fields[0].parse().expect("integer bond cap"),
                    probability: fields[1].parse().expect("measured echo return"),
                    seconds: (!fields[3].is_empty()).then(|| {
                        fields[3].parse::<u64>().expect("elapsed nanoseconds") as f64 / 1e9
                    }),
                    peak_rss_bytes: fields[5].parse().expect("peak RSS bytes"),
                }
            })
            .collect()
    })
}

// The uncapped reference can fall on either side of one at f64 precision.
fn bounded_return(raw: f64) -> f64 {
    if (raw - 1.0).abs() <= ECHO_TOLERANCE {
        1.0
    } else {
        raw.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Objective {
    PeakMemory,
    BondCap,
}

fn target_cap(points: &[Sample], target: f64, objective: Objective) -> Option<&Sample> {
    points
        .iter()
        .filter(|p| bounded_return(p.probability) >= target)
        .min_by_key(|p| match objective {
            Objective::PeakMemory => (p.peak_rss_bytes, p.cap),
            Objective::BondCap => (0, p.cap),
        })
}

fn percent(probability: f64) -> String {
    let pct = bounded_return(probability) * 100.0;
    if pct == 100.0 {
        "≈100%".into()
    } else if pct > 99.9999 {
        ">99.9999%".into()
    } else if pct > 0.0 && pct < 0.01 {
        format!("{pct:.2e}%")
    } else {
        format!("{pct:.4}%")
    }
}

fn fixture_bound() -> &'static quantsim_core::mps_ladder::BondBound {
    static BOUND: OnceLock<quantsim_core::mps_ladder::BondBound> = OnceLock::new();
    BOUND.get_or_init(quantsim_core::mps_ladder::benchmark_bound)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    CircuitBound,
    Target,
    Manual,
}

#[derive(Debug)]
pub(crate) struct LadderWindow {
    pub(crate) open: bool,
    mode: Mode,
    target_percent: f64,
    objective: Objective,
    manual_cap: usize,
    #[cfg(not(target_arch = "wasm32"))]
    run: desktop::EchoRun,
}

impl Default for LadderWindow {
    fn default() -> Self {
        Self {
            open: false,
            mode: Mode::CircuitBound,
            target_percent: 99.0,
            objective: Objective::PeakMemory,
            manual_cap: 32,
            #[cfg(not(target_arch = "wasm32"))]
            run: Default::default(),
        }
    }
}

impl LadderWindow {
    pub(crate) fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let mut open = true;
        egui::Window::new("MPS ladder experiment")
            .open(&mut open)
            .default_width(440.0)
            .min_width(240.0)
            .vscroll(true)
            .show(ctx, |ui| self.content(ui));
        self.open = open;
    }

    fn content(&mut self, ui: &mut Ui) {
        let points = samples();
        ui.label("256 qubits · 2 × 128 ladder · 8 rounds · seed 53");
        ui.label("Echo return is the chance of recovering all zeros after running this ladder forward and backward.");
        ui.horizontal_wrapped(|ui| {
            ui.label("Bond cap");
            ui.selectable_value(&mut self.mode, Mode::CircuitBound, "Auto");
            ui.selectable_value(&mut self.mode, Mode::Target, "Target return");
            ui.selectable_value(&mut self.mode, Mode::Manual, "Manual");
        });
        let bound = fixture_bound();
        let sufficient = usize::try_from(&bound.uniform_cap).ok();
        let selected = match self.mode {
            Mode::CircuitBound => {
                ui.label("Auto uses the gate schedule to bound the rank across every cut.");
                if let Ok(bytes) = u64::try_from(&bound.tensor_bytes) {
                    ui.small(format!(
                        "Retained tensor bound {:.2} MiB",
                        bytes as f64 / 1048576.0
                    ));
                }
                sufficient
            }
            Mode::Target => self.choose_target(ui, points),
            Mode::Manual => {
                egui::ComboBox::from_label("Uniform bond cap")
                    .selected_text(self.manual_cap.to_string())
                    .show_ui(ui, |ui| {
                        for p in points {
                            ui.selectable_value(&mut self.manual_cap, p.cap, p.cap.to_string());
                        }
                    });
                Some(self.manual_cap)
            }
        };
        let sample = points.iter().find(|p| Some(p.cap) == selected);
        curve(
            ui,
            points,
            sample,
            sufficient,
            (self.mode == Mode::Target).then_some(self.target_percent / 100.0),
        );
        if let Some(cap) = selected {
            ui.strong(format!("Selected cap {cap}"));
            if let Some(sample) = sample {
                ui.label(format!("Measured return {}", percent(sample.probability)));
                let mib = sample.peak_rss_bytes as f64 / 1048576.0;
                let time = sample
                    .seconds
                    .map_or(String::new(), |s| format!(" · {s:.2} s"));
                ui.small(format!(
                    "M1 Max reference{time} · {mib:.1} MiB peak process memory"
                ));
            }
        } else if self.mode == Mode::Target {
            ui.label("No measured cap reaches this target.");
        } else {
            ui.label("The inferred cap exceeds the addressable range.");
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.run.controls(ui, selected);
        #[cfg(target_arch = "wasm32")]
        ui.small("Open the desktop app to run the selected cap locally.");
    }
    fn choose_target(&mut self, ui: &mut Ui, points: &[Sample]) -> Option<usize> {
        ui.add(
            egui::Slider::new(&mut self.target_percent, 0.0..=100.0)
                .text("Target return")
                .suffix("%")
                .step_by(0.01),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Prefer");
            ui.selectable_value(&mut self.objective, Objective::PeakMemory, "Lowest memory")
                .on_hover_text(
                    "Lowest measured peak process memory among caps that meet the target.",
                );
            ui.selectable_value(&mut self.objective, Objective::BondCap, "Smallest cap")
                .on_hover_text("Smallest tested bond dimension that meets the target.");
        });
        target_cap(points, self.target_percent / 100.0, self.objective).map(|p| p.cap)
    }
}

fn curve(
    ui: &mut Ui,
    points: &[Sample],
    selected: Option<&Sample>,
    sufficient: Option<usize>,
    target: Option<f64>,
) {
    let log = points.iter().all(|p| p.probability > 0.0);
    let ordinate = |p: f64| if log { p.log10() } else { p };
    let xy = |p: &Sample| [p.cap as f64, ordinate(bounded_return(p.probability))];
    let measured: Vec<_> = points.iter().map(xy).collect();
    let min_y = measured.iter().map(|p| p[1]).fold(0.0, f64::min);
    let max_x = points.iter().map(|p| p.cap).max().unwrap_or(1) as f64;
    let color = ui.visuals().hyperlink_color;
    let marker = ui.visuals().strong_text_color();
    let mut graph = Plot::new("ladder_echo_return")
        .height(152.0)
        .allow_scroll(false)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_axis_zoom_drag(false)
        .allow_boxed_zoom(false)
        .default_x_bounds(0.0, max_x * 1.03)
        .default_y_bounds(
            if log { min_y - 2.0 } else { -0.03 },
            if log { 2.0 } else { 1.03 },
        )
        .x_axis_label("Bond cap")
        .y_axis_label(if log {
            "Return probability (log)"
        } else {
            "Return probability"
        })
        .show_x(false)
        .show_y(false);
    if log {
        graph = graph
            .y_grid_spacer(|_| {
                ((min_y.floor() as i32)..=0)
                    .filter(|n| n % 10 == 0)
                    .map(|n| GridMark {
                        value: n as f64,
                        step_size: 10.0,
                    })
                    .collect()
            })
            .y_axis_formatter(|tick, _| {
                if tick.value == 0.0 {
                    "1".into()
                } else {
                    format!("1e{:.0}", tick.value)
                }
            });
    }
    graph.show(ui, |plot| {
        if let Some(target) = target.filter(|p| !log || *p > 0.0) {
            plot.hline(
                HLine::new("Target", ordinate(target))
                    .style(LineStyle::dashed_dense())
                    .color(marker),
            );
        }
        // Keep the sufficient-cap control separate: intermediate runs can time out.
        let capped: Vec<_> = points
            .iter()
            .filter(|p| Some(p.cap) != sufficient)
            .map(xy)
            .collect();
        plot.line(Line::new("Measured capped echoes", capped).color(color));
        plot.points(
            Points::new("Measured caps", measured)
                .color(color)
                .radius(3.0_f32),
        );
        if let Some(p) = selected {
            plot.points(
                Points::new("Selected cap", vec![xy(p)])
                    .color(marker)
                    .radius(5.0_f32),
            );
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use super::percent;
    use std::{
        thread,
        time::{Duration, Instant},
    };

    #[derive(Debug, Default)]
    pub(super) enum EchoRun {
        #[default]
        Idle,
        Running {
            cap: usize,
            worker: thread::JoinHandle<Result<(f64, Duration), &'static str>>,
        },
        Finished {
            cap: usize,
            probability: f64,
            elapsed: Duration,
        },
        Failed(String),
    }

    impl EchoRun {
        pub(super) fn controls(&mut self, ui: &mut egui::Ui, selected: Option<usize>) {
            if matches!(self, Self::Running { worker, .. } if worker.is_finished()) {
                if let Self::Running { cap, worker } = std::mem::take(self) {
                    *self = match worker.join().unwrap_or(Err("echo worker stopped")) {
                        Ok((probability, elapsed)) => Self::Finished {
                            cap,
                            probability,
                            elapsed,
                        },
                        Err(message) => Self::Failed(format!("Cap {cap}: {message}")),
                    };
                }
            }
            if ui
                .add_enabled(
                    selected.is_some() && !matches!(self, Self::Running { .. }),
                    egui::Button::new("Run echo"),
                )
                .clicked()
            {
                if let Some(cap) = selected {
                    *self = match thread::Builder::new()
                        .name("mps-echo".into())
                        .spawn(move || {
                            let start = Instant::now();
                            let probability = quantsim_core::mps_ladder::run_echo(cap)?;
                            Ok((probability, start.elapsed()))
                        }) {
                        Ok(worker) => Self::Running { cap, worker },
                        Err(error) => Self::Failed(error.to_string()),
                    };
                }
            }
            match self {
                Self::Running { cap, .. } => {
                    ui.label(format!("Running at cap {cap}…"));
                    ui.ctx().request_repaint_after(Duration::from_millis(100));
                }
                Self::Finished {
                    cap,
                    probability,
                    elapsed,
                } => {
                    ui.label(format!(
                        "Run at cap {cap} · return {} · {:.2} s",
                        percent(*probability),
                        elapsed.as_secs_f64()
                    ));
                }
                Self::Failed(message) => {
                    ui.colored_label(ui.visuals().error_fg_color, message.as_str());
                }
                Self::Idle => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_filters_by_return_before_applying_the_selected_objective() {
        let point = |cap, probability, peak_rss_bytes| Sample {
            cap,
            probability,
            seconds: None,
            peak_rss_bytes,
        };
        let points = [point(64, 0.8, 1), point(32, 0.85, 32), point(128, 0.99, 16)];
        assert_eq!(
            target_cap(&points, 0.85, Objective::PeakMemory)
                .unwrap()
                .cap,
            128
        );
        assert_eq!(
            target_cap(&points, 0.85, Objective::BondCap).unwrap().cap,
            32
        );
        assert_eq!(
            target_cap(&points, 0.95, Objective::BondCap).unwrap().cap,
            128
        );
        assert!(target_cap(&points, 1.0, Objective::PeakMemory).is_none());
        assert_eq!(bounded_return(1.0 - 1e-13), 1.0);
        assert!(bounded_return(1.0 - 1e-9) < 1.0);
    }

    #[test]
    fn reference_points_cover_the_roundoff_limited_echo() {
        let points = samples();
        assert!(points.windows(2).all(|w| w[0].cap < w[1].cap));
        assert!(points
            .iter()
            .all(|p| (0.0..=1.0 + 2e-10).contains(&p.probability)));
        let end = points.last().unwrap();
        assert_eq!(end.cap, 256);
        assert!((1.0 - end.probability).abs() < 2e-10);
    }
}
