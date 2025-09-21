use egui::{Color32, Painter, Pos2, Sense, Shape, Stroke, Ui, Vec2};
use nalgebra::{Complex, Matrix2, Vector2};
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
        initial_state: Vector2<Complex<f64>>,
    ) {
        let final_state = gate_matrix * initial_state;

        let (initial_x, initial_y, initial_z) = to_cartesian(initial_state);
        let (final_x, final_y, final_z) = to_cartesian(final_state);

        let animated_x = initial_x + (final_x - initial_x) * t;
        let animated_y = initial_y + (final_y - initial_y) * t;
        let animated_z = initial_z + (final_z - initial_z) * t;

        let (rect, _response) = ui.allocate_exact_size(Vec2::splat(300.0), Sense::hover());

        let painter = ui.painter_at(rect);
        let visuals = ui.visuals().clone();
        self.paint_sphere(
            &painter,
            rect.center(),
            rect.width() / 2.5,
            (animated_x, animated_y, animated_z),
            (initial_x, initial_y, initial_z),
            (final_x, final_y, final_z),
            &visuals,
        );
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
        let wireframe_color = visuals
            .widgets
            .noninteractive
            .fg_stroke
            .color
            .linear_multiply(0.5);
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
            painter.add(Shape::line(points, Stroke::new(1.0, wireframe_color)));
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
            painter.add(Shape::line(points, Stroke::new(1.0, wireframe_color)));
        }

        // Axes
        self.paint_axis(painter, center, radius, (1.0, 0.0, 0.0), "+x", visuals);
        self.paint_axis(painter, center, radius, (-1.0, 0.0, 0.0), "-x", visuals);
        self.paint_axis(painter, center, radius, (0.0, 1.0, 0.0), "+y", visuals);
        self.paint_axis(painter, center, radius, (0.0, -1.0, 0.0), "-y", visuals);
        self.paint_axis(painter, center, radius, (0.0, 0.0, -1.0), "|1>", visuals);
        self.paint_axis(painter, center, radius, (0.0, 0.0, 1.0), "|0>", visuals);

        // Vectors
        self.paint_vector(painter, center, radius, initial_vec, Color32::BLUE);
        self.paint_vector(painter, center, radius, final_vec, Color32::GREEN);
        self.paint_vector(painter, center, radius, animated_vec, Color32::GREEN);
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
    ) {
        let start_point = self.project(center, radius, (0.0, 0.0, 0.0));
        let end_point = self.project(center, radius, vec);
        painter.arrow(
            start_point,
            end_point - start_point,
            Stroke::new(2.0, color),
        );
    }

    fn project(&self, center: Pos2, radius: f32, p: (f64, f64, f64)) -> Pos2 {
        let x = p.0 + 0.4 * p.1;
        let y = p.2 - 0.4 * p.1;
        center + Vec2::new(x as f32, y as f32) * radius
    }
}

fn to_cartesian(state: Vector2<Complex<f64>>) -> (f64, f64, f64) {
    let alpha = state[0];
    let beta = state[1];

    let z = alpha.norm_sqr() - beta.norm_sqr();
    let x = 2.0 * (alpha.conj() * beta).re;
    let y = 2.0 * (alpha.conj() * beta).im;

    (x, y, z)
}
