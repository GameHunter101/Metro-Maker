use core::f32;
use std::collections::BinaryHeap;

use nalgebra::Vector2;
use rand::Rng;
use rayon::prelude::*;

use crate::tensor_field::{EvalEigenvectors, GRID_SIZE, Point, TensorField};

pub fn distribute_points(point_count: u32) -> Vec<Point> {
    let mut rand = rand::rng();

    let mut points = Vec::new();

    while (points.len() as u32) < point_count {
        let candidates: Vec<Point> = (0..10)
            .map(|_| {
                Point::new(
                    rand.random_range(0..GRID_SIZE) as f32,
                    rand.random_range(0..GRID_SIZE) as f32,
                )
            })
            .collect();
        let distances: Vec<f32> = candidates
            .iter()
            .map(|point| closest_distance_to_points(*point, &points))
            .collect();
        let farthest_point_index = distances
            .iter()
            .enumerate()
            .fold((0, distances[0]), |acc, (i, dist)| {
                if *dist > acc.1 { (i, *dist) } else { acc }
            })
            .0;

        points.push(candidates[farthest_point_index]);
    }

    points
}

fn closest_distance_to_points(candidate: Point, points: &[Point]) -> f32 {
    points
        .iter()
        .fold(f32::MAX, |acc, p| {
            let dist_squared = (p - candidate).magnitude_squared();
            if dist_squared < acc {
                dist_squared
            } else {
                acc
            }
        })
        .sqrt()
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct SeedPoint {
    pub seed: Point,
    pub priority: f32,
    pub follow_major_eigenvectors: bool,
}

impl Eq for SeedPoint {}

impl Ord for SeedPoint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.total_cmp(&other.priority)
    }
}

pub fn prioritize_points(
    points: &[Point],
    city_center: Point,
    tensor_field: &TensorField,
) -> BinaryHeap<SeedPoint> {
    points
        .iter()
        .flat_map(|point| {
            let eigenvectors = tensor_field.evaluate_smoothed_field_at_point(*point);
            if eigenvectors.norm() <= 0.0001 {
                None
            } else {
                let city_center_priority = (-(city_center - point).magnitude()).exp();
                let degenerate_point_priority =
                    (-closest_degenerate_point_distance(*point, tensor_field)).exp();

                Some(SeedPoint {
                    seed: *point,
                    priority: city_center_priority + degenerate_point_priority,
                    follow_major_eigenvectors: true,
                })
            }
        })
        .collect()
}

/// Divide the full grid into rectangular sectors. Search each sector individually for a degenerate
/// point starting with the sectors closest to the queried point. The distance to the center of the closest sector with a
/// degenerate point is returned. Precision is not essential, so the centers should be good enough.
/// More sectors can help with precision, but decrease performance
fn closest_degenerate_point_distance(point: Point, tensor_field: &TensorField) -> f32 {
    let horizontal_sector_count = 16;
    let vertical_sector_count = horizontal_sector_count;

    let horizontal_sector_size = GRID_SIZE / horizontal_sector_count;
    let vertical_sector_size = GRID_SIZE / vertical_sector_count;

    let mut sector_order = (0..horizontal_sector_count)
        .flat_map(|x| {
            (0..vertical_sector_count)
                .map(|y| {
                    Point::new(
                        (x * horizontal_sector_size) as f32 + horizontal_sector_size as f32 / 2.0,
                        (y * vertical_sector_size) as f32 + vertical_sector_size as f32 / 2.0,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    sector_order.sort_by(|a, b| {
        (a - point)
            .norm_squared()
            .total_cmp(&(b - point).norm_squared())
    });

    (sector_order
        .into_iter()
        .filter(|sector_center| {
            let x_sector = (sector_center.x - horizontal_sector_size as f32 / 2.0)
                / horizontal_sector_size as f32;
            let y_sector =
                (sector_center.y - vertical_sector_size as f32 / 2.0) / vertical_sector_size as f32;

            sector_has_degenerate_point(
                x_sector as u32,
                y_sector as u32,
                horizontal_sector_size,
                vertical_sector_size,
                tensor_field,
            )
        })
        .next()
        .unwrap_or(Point::new(f32::MAX, f32::MAX))
        - point)
        .norm_squared()
}

fn sector_has_degenerate_point(
    x_sector: u32,
    y_sector: u32,
    horizontal_sector_size: u32,
    vertical_sector_size: u32,
    tensor_field: &TensorField,
) -> bool {
    for x_in_sector in 0..horizontal_sector_size {
        for y_in_sector in 0..vertical_sector_size {
            let x = x_in_sector + x_sector * horizontal_sector_size;
            let y = y_in_sector + y_sector * vertical_sector_size;
            if tensor_field
                .evaluate_smoothed_field_at_point(Point::new(x as f32, y as f32))
                .norm()
                <= 0.0001
            {
                return true;
            }
        }
    }

    return false;
}

pub enum TraceSeeds {
    Random(u32),
    Specific(Vec<SeedPoint>),
}

pub fn heap_to_vec<T: Clone + Ord>(heap: BinaryHeap<T>) -> Vec<T> {
    let mut res = Vec::new();
    let mut heap = heap.clone();
    while let Some(head) = heap.pop() {
        res.push(head);
    }
    res
}

pub fn trace_street_plan(
    tensor_field: &TensorField,
    seeds: TraceSeeds,
    city_center: Point,
    d_sep: f32,
    iter_count: usize,
    previous_major_curves: Vec<HermiteCurve>,
    previous_minor_curves: Vec<HermiteCurve>,
) -> (Vec<Vec<ControlPoint>>, Vec<Vec<ControlPoint>>) {
    let mut seed_points = match seeds {
        TraceSeeds::Random(starting_seed_count) => {
            let temp_points = [
                (391.0, 113.0),
                (10.0, 470.0),
                (382.0, 472.0),
                (61.0, 152.0),
                (413.0, 291.0),
                (191.0, 298.0),
                (0.0, 303.0),
                (147.0, 0.0),
                (304.0, 294.0),
                (298.0, 41.0),
                (230.0, 509.0),
                (502.0, 416.0),
                (127.0, 205.0),
                (285.0, 162.0),
                (459.0, 40.0),
                (299.0, 436.0),
                (121.0, 472.0),
                (508.0, 493.0),
                (470.0, 151.0),
                (214.0, 413.0),
                (364.0, 355.0),
                (171.0, 63.0),
                (355.0, 191.0),
                (274.0, 355.0),
                (66.0, 336.0),
                (230.0, 65.0),
                (30.0, 31.0),
                (223.0, 12.0),
                (193.0, 146.0),
                (447.0, 224.0),
            ]
            .map(|p| Point::new(p.0, p.1));

            prioritize_points(
                // &distribute_points(starting_seed_count),
                &temp_points,
                city_center,
                &tensor_field,
            )
        }
        TraceSeeds::Specific(seed_points) => BinaryHeap::from(seed_points),
    };

    let prev_major_len = previous_major_curves.len();
    let mut major_curves = previous_major_curves;
    let prev_minor_len = previous_minor_curves.len();
    let mut minor_curves = previous_minor_curves;

    let d_sep_val = d_sep;

    for i in 0..iter_count {
        let h = 0.2;
        let d_sep = |_point: Point| d_sep_val/*  + (point - city_center).norm() / GRID_SIZE as f32 * 15.0 */;
        let follow_major_eigenvectors = (i % 2) == 0;

        let traces = trace_lanes(
            heap_to_vec(seed_points.clone())
                .iter()
                .enumerate()
                .map(|(i, seed)| (i, seed.seed))
                .collect::<Vec<_>>(),
            tensor_field,
            h,
            d_sep.clone(),
            follow_major_eigenvectors,
            200.0,
            if follow_major_eigenvectors {
                &major_curves
            } else {
                &minor_curves
            },
        );

        let pre: usize = traces
            .iter()
            .map(|TraceOutput { new_seeds, .. }| new_seeds.len())
            .sum();

        let curve_paths = smooth_lanes(traces, 0.03, 0.3, 20, h, 0.7);

        let (clipped_paths, new_seeds): (Vec<HermiteCurve>, Vec<Vec<Point>>) = clip_pass(
            curve_paths,
            if follow_major_eigenvectors {
                &major_curves
            } else {
                &minor_curves
            },
            d_sep,
            d_sep_val * 2.0,
        )
        .into_iter()
        .unzip();

        println!("Pre: {pre}, post: {}", new_seeds.len());

        seed_points.extend(new_seeds.into_iter().flatten().map(|seed| SeedPoint {
            seed,
            priority: 0.0,
            follow_major_eigenvectors,
        }));

        if follow_major_eigenvectors {
            major_curves.extend(clipped_paths);
        } else {
            minor_curves.extend(clipped_paths);
        }
    }

    (
        major_curves[prev_major_len..].to_vec(),
        minor_curves[prev_major_len..].to_vec(),
    )
}

fn trace_lanes(
    seeds: Vec<(usize, Point)>,
    tensor_field: &TensorField,
    h: f32,
    d_sep: impl Fn(Point) -> f32 + Send + Sync + Clone,
    follow_major_eigenvectors: bool,
    max_len: f32,
    previous_curves: &[HermiteCurve],
) -> Vec<TraceOutput> {
    let unordered_traces: Vec<(usize, TraceOutput)> = seeds
        .into_par_iter()
        .map(|(index, seed)| {
            let d_sep = d_sep.clone();
            (
                index,
                trace(
                    tensor_field,
                    seed,
                    h,
                    d_sep,
                    follow_major_eigenvectors,
                    max_len,
                    previous_curves,
                ),
            )
        })
        .collect();

    let mut ordered_traces = vec![
        TraceOutput {
            path: Vec::new(),
            new_seeds: Vec::new()
        };
        unordered_traces.len()
    ];

    for (index, trace_output) in unordered_traces {
        ordered_traces[index] = trace_output;
    }

    ordered_traces
}

#[derive(Debug, Clone, Default)]
struct TraceOutput {
    path: Vec<Point>,
    new_seeds: Vec<(Point, f32)>,
}

fn trace(
    tensor_field: &TensorField,
    seed: Point,
    h: f32,
    d_sep: impl Fn(Point) -> f32,
    follow_major_eigenvectors: bool,
    max_len: f32,
    previous_curves: &[HermiteCurve],
) -> TraceOutput {
    let origin = seed;
    let mut seed = seed;
    let mut path = vec![seed];
    let mut accumulated_distance = 0.0;
    let mut distance_since_last_seed = 0.0;
    let mut closest_distance_to_curves =
        raycast_to_curve(seed, previous_curves).sqrt() - d_sep(seed);
    let mut distance_since_last_distance_check = 0.0;
    let mut new_seeds = vec![];
    let mut steps = 0;

    let clamp_vel = |vel: Vector2<f32>| {
        clamp_vector_between_points(
            Vector2::new(-(GRID_SIZE as f32), -(GRID_SIZE as f32)),
            Vector2::new(GRID_SIZE as f32, GRID_SIZE as f32),
            vel,
        )
    };

    if closest_distance_to_curves <= 0.0 {
        return TraceOutput::default();
    }

    while !(seed.x < 0.0 || seed.y < 0.0 || seed.x > GRID_SIZE as f32 || seed.y > GRID_SIZE as f32)
    {
        let tensor = tensor_field.evaluate_smoothed_field_at_point(seed);

        if tensor.norm() < 0.0001 {
            break;
        }

        let k_1_eigenvectors = tensor.eigenvectors();
        let k_1 = branchless_if(
            clamp_vel(k_1_eigenvectors.major),
            clamp_vel(k_1_eigenvectors.minor),
            follow_major_eigenvectors,
        )
        .normalize();
        let k_2_eigenvectors = tensor_field
            .evaluate_smoothed_field_at_point(clamp_vec_to_grid(seed + h / 2.0 * k_1))
            .eigenvectors();
        let k_2 = branchless_if(
            clamp_vel(k_2_eigenvectors.major),
            clamp_vel(k_2_eigenvectors.minor),
            follow_major_eigenvectors,
        )
        .normalize();
        let k_3_eigenvectors = tensor_field
            .evaluate_smoothed_field_at_point(clamp_vec_to_grid(seed + h / 2.0 * k_2))
            .eigenvectors();
        let k_3 = branchless_if(
            clamp_vel(k_3_eigenvectors.major),
            clamp_vel(k_3_eigenvectors.minor),
            follow_major_eigenvectors,
        )
        .normalize();
        let k_4_eigenvectors = tensor_field
            .evaluate_smoothed_field_at_point(clamp_vec_to_grid(seed + h * k_3))
            .eigenvectors();
        let k_4 = branchless_if(
            clamp_vel(k_4_eigenvectors.major),
            clamp_vel(k_4_eigenvectors.minor),
            follow_major_eigenvectors,
        )
        .normalize();

        let m = 1.0 / 6.0 * k_1 + 1.0 / 3.0 * k_2 + 1.0 / 3.0 * k_3 + 1.0 / 6.0 * k_4;

        let new_pos = seed + h * m;
        let dist = (new_pos - seed).norm();

        accumulated_distance += dist;
        distance_since_last_seed += dist;
        distance_since_last_distance_check += dist;

        if distance_since_last_seed >= d_sep(new_pos) {
            distance_since_last_seed = 0.0;
            new_seeds.push((new_pos, accumulated_distance));
        }
        seed = new_pos;

        if distance_since_last_distance_check >= closest_distance_to_curves {
            let new_closest_distance =
                raycast_to_curve(new_pos, previous_curves).sqrt() - d_sep(new_pos);
            if new_closest_distance <= 0.0 {
                break;
            } else {
                closest_distance_to_curves = new_closest_distance;
                distance_since_last_distance_check = 0.0;
            }
        }

        if h < 1.0 {
            if steps as f32 % (1.0 / h) <= 0.001 {
                path.push(new_pos);
            }
        }
        steps += 1;

        if (new_pos - origin).magnitude_squared() <= 0.0001 || accumulated_distance > max_len {
            break;
        }
    }

    let new_seeds = new_seeds
        .into_iter()
        .map(|(e, f)| (e, f / accumulated_distance))
        .collect();

    if path.len() < 2 {
        TraceOutput::default()
    } else {
        TraceOutput { path, new_seeds }
    }
}

fn branchless_if<T: std::ops::Mul<f32, Output = T> + std::ops::Add<T, Output = T>>(
    true_val: T,
    false_val: T,
    condition: bool,
) -> T {
    true_val * (condition as i32) as f32 + false_val * (1 - condition as i32) as f32
}

fn clamp_vector_between_points(p_0: Point, p_1: Point, subject: Point) -> Point {
    let max_point = Point::new(p_0.x.max(p_1.x), p_0.y.max(p_1.y));
    let min_point = Point::new(p_0.x.min(p_1.x), p_0.y.min(p_1.y));

    Point::new(
        subject.x.clamp(min_point.x, max_point.x),
        subject.y.clamp(min_point.y, max_point.y),
    )
}

fn clamp_vec_to_grid(vec: Vector2<f32>) -> Vector2<f32> {
    let clamped_x = vec.x.clamp(0.0, GRID_SIZE as f32);
    let clamped_y = vec.y.clamp(0.0, GRID_SIZE as f32);
    Vector2::new(
        if clamped_x.is_nan() { 1.0 } else { clamped_x },
        if clamped_y.is_nan() { 1.0 } else { clamped_y },
    )
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ControlPoint {
    pub position: Point,
    pub velocity: Vector2<f32>,
}

type HermiteCurve = Vec<ControlPoint>;

struct SmoothedCurve {
    curve: HermiteCurve,
    new_seeds: Vec<(Point, f32)>,
}

fn smooth_lanes(
    traces: Vec<TraceOutput>,
    alpha: f32,
    beta: f32,
    point_side_padding: usize,
    h: f32,
    blend_factor: f32,
) -> Vec<SmoothedCurve> {
    let h_square = h * h;
    traces
        .into_par_iter()
        .filter(|TraceOutput { path, .. }| !path.is_empty())
        .map(|TraceOutput { path, new_seeds }| {
            let smoothed_path = smooth_path(path.clone(), alpha, beta);
            if smoothed_path.len() == 1 {
                println!("Prev: {}", path.clone().len());
            }

            let mut control_points_indices: Vec<usize> =
                highest_curvature_points(&smoothed_path, point_side_padding);

            control_points_indices.sort();

            let curve = control_points_indices
                .iter()
                .map(|&index| {
                    let position = smoothed_path[index];
                    let velocity = if index == 0 {
                        (smoothed_path[1] - position) / h_square
                    } else if index == smoothed_path.len() - 1 {
                        (smoothed_path[smoothed_path.len() - 1]
                            - smoothed_path[smoothed_path.len() - 2])
                            / h_square
                    } else {
                        blend_factor
                            * ((smoothed_path[index + 1] - position) / h_square
                                + (position - smoothed_path[index - 1]) / h_square)
                    };

                    ControlPoint { position, velocity }
                })
                .collect::<Vec<_>>();

            SmoothedCurve { curve, new_seeds }
        })
        .collect()
}

pub fn smooth_path(path: Vec<Point>, alpha: f32, beta: f32) -> Vec<Point> {
    let first_pass: Vec<Point> = path
        .iter()
        .enumerate()
        .map(|(i, point)| {
            if i == 0 || i == path.len() - 1 {
                *point
            } else {
                let current_vel = point_first_deriv(i, &path);
                let prev_vel = point_first_deriv(i - 1, &path);

                let current_vel_normed = point_first_deriv(i, &path).normalize();
                let prev_vel_normed = point_first_deriv(i - 1, &path).normalize();

                if ((1.0 - current_vel_normed.dot(&prev_vel_normed).min(1.0)).min(alpha) / alpha)
                    .floor()
                    == 0.0
                {
                    *point
                } else {
                    point
                        + (current_vel - prev_vel) / 2.0
                            * (current_vel.norm_squared() / (current_vel - prev_vel).norm_squared())
                                .sqrt()
                }
            }
        })
        .collect();

    first_pass
        .iter()
        .enumerate()
        .map(|(i, point)| {
            if i == 0 || i == first_pass.len() - 1 {
                *point
            } else if ((1.0 - beta)
                + (point_first_deriv(i, &first_pass).normalize()
                    - point_first_deriv(i - 1, &first_pass).normalize())
                .norm())
            .floor()
                > 0.0
            {
                (first_pass[i - 1] + point + first_pass[i + 1]) / 3.0
            } else {
                *point
            }
        })
        .collect()
}

fn point_first_deriv(point_index: usize, path: &[Point]) -> Vector2<f32> {
    path[point_index + 1] - path[point_index]
}

pub fn point_second_deriv(point_index: usize, path: &[Point]) -> Vector2<f32> {
    point_first_deriv(point_index, path).normalize()
        - point_first_deriv(point_index - 1, path).normalize()
}

pub fn resample_curve(control_points: Vec<ControlPoint>, points_per_spline: i32) -> Vec<Point> {
    let sampler = |p_0: Point, p_1: Point, m_0: Vector2<f32>, m_1: Vector2<f32>| -> Vec<Point> {
        (0..=points_per_spline)
            .map(|i| {
                evaluate_hermite_curve(p_0, p_1, m_0, m_1, i as f32 / points_per_spline as f32)
            })
            .collect()
    };

    (0..control_points.len() - 1)
        .flat_map(|i| {
            let this_control_point = control_points[i];
            let next_control_point = control_points[i + 1];
            sampler(
                this_control_point.position,
                next_control_point.position,
                this_control_point.velocity,
                next_control_point.velocity,
            )
        })
        .collect()
}

pub fn highest_curvature_points(path: &[Point], point_padding_per_side: usize) -> Vec<usize> {
    let mut indices_sorted_by_curvature: Vec<usize> = (1..path.len() - 1).collect();

    indices_sorted_by_curvature.sort_by(|&a, &b| {
        point_second_deriv(a, path)
            .norm_squared()
            .partial_cmp(&point_second_deriv(b, path).norm_squared())
            .unwrap()
            .reverse()
    });
    let mut curvature_point_indices = vec![0, path.len() - 1];
    indices_sorted_by_curvature
        .into_iter()
        .filter(|&index| {
            for curvature_index in &curvature_point_indices {
                if curvature_index.abs_diff(index) < point_padding_per_side {
                    return false;
                }
            }
            curvature_point_indices.push(index);
            true
        })
        .chain(vec![0, path.len() - 1])
        .collect()
}

fn clip_pass(
    curve_paths: Vec<SmoothedCurve>,
    previous_curves: &[HermiteCurve],
    d_sep: impl Fn(Point) -> f32,
    min_length: f32,
) -> Vec<(HermiteCurve, Vec<Point>)> {
    (1..curve_paths.len())
        .flat_map(|curve_index| {
            let SmoothedCurve {
                curve: current_curve,
                new_seeds,
            } = &curve_paths[curve_index];

            let prev_curves_as_hermite: Vec<HermiteCurve> = curve_paths[..curve_index]
                .iter()
                .map(|curve| curve.curve.clone())
                .chain(previous_curves.iter().cloned())
                .collect();

            if raycast_to_curve(current_curve[0].position, &prev_curves_as_hermite)
                < (0.85 * d_sep(current_curve[0].position) * d_sep(current_curve[0].position))
            {
                return None;
            }

            let control_point_distances_squared: Vec<f32> = current_curve
                .iter()
                .map(|control_point| {
                    raycast_to_curve(control_point.position, &prev_curves_as_hermite)
                })
                .collect();

            let mut clipped_index = 0;
            for (i, dist_squared) in control_point_distances_squared.iter().enumerate() {
                let d_sep_val = d_sep(current_curve[i].position);
                if *dist_squared >= d_sep_val * d_sep_val {
                    clipped_index += 1;
                } else {
                    break;
                }
            }

            if clipped_index < 2 {
                return None;
            }

            let filtered: HermiteCurve = current_curve[..clipped_index].iter().copied().collect();

            let clipped_length =
                filtered[1..]
                    .iter()
                    .enumerate()
                    .fold(0.0, |acc, (i, control_point)| {
                        acc + (control_point.position - filtered[i].position).norm()
                    });

            if clipped_length < min_length {
                return None;
            }

            if filtered.is_empty() {
                None
            } else {
                Some((
                    filtered,
                    new_seeds
                        .iter()
                        .flat_map(|(seed, t)| {
                            if *t < (clipped_index as f32 / current_curve.len() as f32) {
                                Some(*seed)
                            } else {
                                None
                            }
                        })
                        .collect(),
                ))
            }
        })
        .collect()
}

/// Returns the squared distance to the nearest point to save on computation.
/// Raycasts to a piecewise linear approximation of the hermite curves. This isn't completely
/// accurate but the curve fit is good enough where the error is negligible
fn raycast_to_curve(point: Point, other_curves: &[HermiteCurve]) -> f32 {
    other_curves
        .par_iter()
        .map(|curve| {
            (0..curve.len() - 1)
                .map(|start_index| {
                    let p_0 = curve[start_index].position;
                    let p_1 = curve[start_index + 1].position;
                    let segment_vector = p_1 - p_0;
                    let projection = project_vector(point - p_0, segment_vector);

                    (point - clamp_vector_between_points(p_0, p_1, p_0 + projection)).norm_squared()
                })
                .reduce(|acc, e| if e < acc { e } else { acc })
                .unwrap()
        })
        .reduce(|| f32::MAX, |acc, e| if e < acc { e } else { acc })
}

/// Returns the squared distance to the nearest point to save on computation.
/// Raycasts to a piecewise linear approximation of the hermite curves. This isn't completely
/// accurate but the curve fit is good enough where the error is negligible.
/// This function also returns the closest approximation point, and the velocity at that approximation
fn raycast_to_curve_with_more_info(
    point: Point,
    other_curves: &[HermiteCurve],
) -> (f32, Point, Vector2<f32>) {
    other_curves
        .par_iter()
        .map(|curve| {
            (0..curve.len() - 1)
                .map(|start_index| {
                    let p_0 = curve[start_index].position;
                    let p_1 = curve[start_index + 1].position;
                    let segment_vector = p_1 - p_0;
                    let projection = project_vector(point - p_0, segment_vector);

                    let clamped_projection =
                        clamp_vector_between_points(p_0, p_1, p_0 + projection);

                    (
                        (point - clamped_projection).norm_squared(),
                        clamped_projection,
                        segment_vector,
                    )
                })
                .reduce(|acc, e| if e < acc { e } else { acc })
                .unwrap()
        })
        .reduce(
            || (f32::MAX, Point::zeros(), Vector2::zeros()),
            |acc, e| if e.0 < acc.0 { e } else { acc },
        )
}

fn project_vector(subject: Point, target: Vector2<f32>) -> Point {
    subject.dot(&target) / target.norm_squared() * target
}

fn evaluate_hermite_curve(
    p_0: Point,
    p_1: Point,
    m_0: Vector2<f32>,
    m_1: Vector2<f32>,
    t: f32,
) -> Point {
    t * (t * (t * (2.0 * (p_0 - p_1) + m_0 + m_1) - 2.0 * m_0 - m_1 - 3.0 * (p_0 - p_1)) + m_0)
        + p_0
}

pub fn merge_road_endings(
    original_curves: &[HermiteCurve],
    curves_to_project_onto: &[HermiteCurve],
    connection_distance: f32,
) -> Vec<HermiteCurve> {
    original_curves
        .par_iter()
        .map(|curve| {
            let mut new_curve = Vec::new();

            let (
                start_point_closest_distance_squared,
                start_point_closest_point,
                _start_point_closest_velocity,
            ) = raycast_to_curve_with_more_info(curve[0].position, &curves_to_project_onto);

            let start_point_closest_distance = start_point_closest_distance_squared.sqrt();

            if start_point_closest_distance > 0.001
                && start_point_closest_distance < connection_distance
            {
                new_curve.push(ControlPoint {
                    position: start_point_closest_point,
                    velocity: curve[0].velocity,
                });
                new_curve.extend(&curve[1..]);
            } else {
                new_curve = curve.clone();
            }

            let (
                end_point_closest_distance_squared,
                end_point_closest_point,
                _end_point_closest_velocity,
            ) = raycast_to_curve_with_more_info(
                curve[curve.len() - 1].position,
                &curves_to_project_onto,
            );

            let end_point_closest_distance = end_point_closest_distance_squared.sqrt();

            if end_point_closest_distance > 0.001
                && end_point_closest_distance < connection_distance
            {
                new_curve.remove(new_curve.len() - 1);
                new_curve.push(ControlPoint {
                    position: end_point_closest_point,
                    velocity: curve[curve.len() - 1].velocity,
                });
            } else {
                new_curve = curve.clone();
            }

            if new_curve.len() < 2 {
                dbg!(new_curve.len());
            }

            new_curve
        })
        .collect()
}
