use egui::{Color32, Frame, Painter, Pos2, RichText, Sense, Shape, Stroke, Ui, Vec2};
use nalgebra::{Complex, Matrix2};
use std::f64::consts::PI;

/// A component for visualizing the Bloch sphere representation of a qubit.
#[derive(Debug, Clone)]
pub struct BlochSphere;

impl Default for BlochSphere {
    fn default() -> Self {
        Self::new()
    }
}

impl BlochSphere {
    /// Creates a new `BlochSphere` component.
    pub fn new() -> Self {
        Self
    }

    /// Draws the Bloch sphere and the state vector of a qubit.
    ///
    /// # Arguments
    ///
    /// * `ui` - The `egui` user interface.
    /// * `gate_matrix` - The gate matrix to apply to the state vector.
    /// * `t` - The animation parameter, from 0.0 to 1.0.
    /// * `initial_state` - The initial state vector of the qubit.
    pub fn draw(
        &self,
        ui: &mut Ui,
        gate_matrix: &Matrix2<Complex<f64>>,
        t: f64,
        initial_density: Matrix2<Complex<f64>>,
    ) {
        let final_density = gate_matrix * initial_density * gate_matrix.adjoint();

        let (initial_x, initial_y, initial_z) = density_to_cartesian(initial_density);
        let (final_x, final_y, final_z) = density_to_cartesian(final_density);

        let animated_x = initial_x + (final_x - initial_x) * t;
        let animated_y = initial_y + (final_y - initial_y) * t;
        let animated_z = initial_z + (final_z - initial_z) * t;

        Frame::group(ui.style())
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(10.0)
            .show(ui, |ui| {
                let (rect, _response) =
                    ui.allocate_exact_size(Vec2::new(320.0, 280.0), Sense::hover());

                let painter = ui.painter_at(rect);
                let visuals = ui.visuals().clone();
                self.paint_sphere(
                    &painter,
                    rect.center_top() + Vec2::new(0.0, 130.0),
                    98.0,
                    (animated_x, animated_y, animated_z),
                    (initial_x, initial_y, initial_z),
                    (final_x, final_y, final_z),
                    &visuals,
                );

                ui.add_space(288.0);
                ui.label(RichText::new("State Coordinates").strong());
                ui.small(format!(
                    "Initial  ({:+.3}, {:+.3}, {:+.3})",
                    initial_x, initial_y, initial_z
                ));
                ui.small(format!(
                    "Final    ({:+.3}, {:+.3}, {:+.3})",
                    final_x, final_y, final_z
                ));
                ui.small(format!(
                    "Animated ({:+.3}, {:+.3}, {:+.3})",
                    animated_x, animated_y, animated_z
                ));
                let (theta, phi) = bloch_angles((final_x, final_y, final_z));
                ui.small(format!("theta = {:.1} deg, phi = {:.1} deg", theta, phi));
                ui.small(format!("Purity(initial) = {:.3}", density_purity(initial_density)));
                ui.small(format!("Purity(final) = {:.3}", density_purity(final_density)));
                render_density_matrix_summary(ui, "rho_in", initial_density);
                render_density_matrix_summary(ui, "rho_out", final_density);
            });
    }

    fn paint_sphere(
        &self,
        painter: &Painter,
        center: Pos2,
        radius: f32,
        animated_vec: (f64, f64, f64),
        initial_vec: (f64, f64, f64),
        final_vec: (f64, f64, f64),
        visuals: &egui::Visuals,
    ) {
        // Faint latitude and longitude lines
        painter.circle_filled(center, radius, visuals.extreme_bg_color.linear_multiply(0.7));
        painter.circle_filled(
            center + Vec2::new(radius * 0.12, -radius * 0.18),
            radius * 0.78,
            Color32::from_white_alpha(10),
        );
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.5, visuals.widgets.noninteractive.bg_stroke.color),
        );

        let wireframe_color = visuals
            .widgets
            .noninteractive
            .fg_stroke
            .color
            .linear_multiply(0.5);
        let hidden_wireframe = wireframe_color.linear_multiply(0.35);
        for i in 0..=10 {
            let mut points = Vec::new();
            for j in 0..=20 {
                let lon = (j as f64 / 20.0) * 2.0 * PI;
                let lat = (i as f64 / 10.0) * PI - PI / 2.0;
                points.push(self.project(
                    center,
                    radius,
                    (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()),
                ));
            }
            let stroke = if i < 5 {
                hidden_wireframe
            } else {
                wireframe_color
            };
            painter.add(Shape::line(points, Stroke::new(1.0, stroke)));
        }
        for i in 0..10 {
            let mut points = Vec::new();
            for j in 0..=20 {
                let lon = (i as f64 / 10.0) * 2.0 * PI;
                let lat = (j as f64 / 20.0) * PI - PI / 2.0;
                points.push(self.project(
                    center,
                    radius,
                    (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()),
                ));
            }
            painter.add(Shape::line(points, Stroke::new(1.0, hidden_wireframe)));
        }

        // Axes
        self.paint_axis(painter, center, radius, (1.0, 0.0, 0.0), "+x", visuals);
        self.paint_axis(painter, center, radius, (-1.0, 0.0, 0.0), "-x", visuals);
        self.paint_axis(painter, center, radius, (0.0, 1.0, 0.0), "+y", visuals);
        self.paint_axis(painter, center, radius, (0.0, -1.0, 0.0), "-y", visuals);
        self.paint_axis(painter, center, radius, (0.0, 0.0, -1.0), "|1>", visuals);
        self.paint_axis(painter, center, radius, (0.0, 0.0, 1.0), "|0>", visuals);

        // Vectors
        self.paint_vector(painter, center, radius, initial_vec, Color32::from_rgb(80, 150, 255), 1.8);
        self.paint_vector(painter, center, radius, final_vec, Color32::from_rgb(70, 220, 140), 2.4);
        self.paint_vector(
            painter,
            center,
            radius,
            animated_vec,
            Color32::from_rgb(255, 210, 90),
            2.8,
        );
    }

    fn paint_axis(
        &self,
        painter: &Painter,
        center: Pos2,
        radius: f32,
        axis: (f64, f64, f64),
        label: &str,
        visuals: &egui::Visuals,
    ) {
        let start = self.project(center, radius, (0.0, 0.0, 0.0));
        let end = self.project(center, radius, axis);
        let axis_color = visuals
            .widgets
            .noninteractive
            .fg_stroke
            .color
            .linear_multiply(0.5);
        let text_color = visuals.widgets.noninteractive.fg_stroke.color;
        painter.line_segment([start, end], Stroke::new(1.5, axis_color));
        painter.text(
            end + Vec2::new(5.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::default(),
            text_color,
        );
    }

    fn paint_vector(
        &self,
        painter: &Painter,
        center: Pos2,
        radius: f32,
        vec: (f64, f64, f64),
        color: Color32,
        width: f32,
    ) {
        let start_point = self.project(center, radius, (0.0, 0.0, 0.0));
        let end_point = self.project(center, radius, vec);
        painter.circle_filled(end_point, 4.0, color);
        painter.arrow(
            start_point,
            end_point - start_point,
            Stroke::new(width, color),
        );
    }

    fn project(&self, center: Pos2, radius: f32, p: (f64, f64, f64)) -> Pos2 {
        let x = p.0 + 0.4 * p.1;
        let y = p.2 - 0.4 * p.1;
        center + Vec2::new(x as f32, y as f32) * radius
    }
}

fn density_to_cartesian(density: Matrix2<Complex<f64>>) -> (f64, f64, f64) {
    let x = 2.0 * density[(0, 1)].re;
    let y = 2.0 * density[(1, 0)].im;
    let z = density[(0, 0)].re - density[(1, 1)].re;
    (x, y, z)
}

fn bloch_angles((x, y, z): (f64, f64, f64)) -> (f64, f64) {
    let theta = z.clamp(-1.0, 1.0).acos().to_degrees();
    let phi = y.atan2(x).to_degrees();
    (theta, phi)
}

fn format_complex(value: Complex<f64>) -> String {
    if value.im.abs() < 1e-6 {
        format!("{:+.3}", value.re)
    } else if value.re.abs() < 1e-6 {
        format!("{:+.3}i", value.im)
    } else {
        format!("{:+.3}{:+.3}i", value.re, value.im)
    }
}

fn density_purity(density: Matrix2<Complex<f64>>) -> f64 {
    let rho_sq = density * density;
    (rho_sq[(0, 0)] + rho_sq[(1, 1)]).re
}

fn render_density_matrix_summary(ui: &mut Ui, label: &str, density: Matrix2<Complex<f64>>) {
    ui.small(format!(
        "{} = [[{}, {}], [{}, {}]]",
        label,
        format_complex(density[(0, 0)]),
        format_complex(density[(0, 1)]),
        format_complex(density[(1, 0)]),
        format_complex(density[(1, 1)]),
    ));
}
