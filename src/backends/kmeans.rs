/*

Imagemagick Replacement:
Random-init k=16 centroids from pixel sample
Iterate assign→recompute until convergence (max 10 iters)
Return centroids sorted by luminance
Crate: image for decode, pure Rust math for clustering. Optional: kmeans-colors crate.

*/

use rayon::prelude::*;
use rand::seq::SliceRandom;

use crate::colors::types::Rgb;
use crate::error::RwalError;
use super::Backend;

pub struct KMeans;

impl Backend for KMeans {
    fn name(&self) -> &str { "accurate" }

    fn generate(&self, pixels: &[Rgb], count: usize, iterations: u8) -> Result<Vec<Rgb>, RwalError> {
        if pixels.is_empty() {
            return Err(RwalError::NoColorsExtracted);
        }

        let k = count.min(pixels.len());
        let mut centroids = init_centroids(pixels, k);

        for _ in 0..iterations {
            // Assign each pixel to nearest centroid (parallel)
            let assignments: Vec<usize> = pixels
                .par_iter()
                .map(|px| nearest_centroid(px, &centroids))
                .collect();

            // Recompute centroids from assigned pixels
            let new_centroids = recompute_centroids(pixels, &assignments, k);

            // Check convergence — stop early if nothing moved
            if centroids_converged(&centroids, &new_centroids) {
                centroids = new_centroids;
                break;
            }

            centroids = new_centroids;
        }

        // Filter out any empty clusters (can happen with small pixel sets)
        let result: Vec<Rgb> = centroids
            .into_iter()
            .filter(|c| *c != Rgb::new(0, 0, 0) || pixels.iter().any(|p| *p == Rgb::new(0, 0, 0)))
            .collect();

        if result.is_empty() {
            return Err(RwalError::NoColorsExtracted);
        }

        Ok(result)
    }
}

/// K-Means++ initialization: pick first centroid randomly, then pick subsequent
/// centroids with probability proportional to squared distance from nearest existing centroid.
fn init_centroids(pixels: &[Rgb], k: usize) -> Vec<Rgb> {
    let mut rng = rand::thread_rng();
    let mut chosen = Vec::with_capacity(k);

    if pixels.is_empty() {
        return chosen;
    }

    chosen.push(*pixels.choose(&mut rng).unwrap());

    let mut min_sq_dists: Vec<u32> = pixels
        .iter()
        .map(|px| squared_distance(px, &chosen[0]))
        .collect();

    while chosen.len() < k {
        let dist = match rand::distributions::WeightedIndex::new(&min_sq_dists) {
            Ok(dist) => dist,
            Err(_) => {
                // All remaining pixels are identical to chosen centroids.
                // Just pad with the first pixel to reach k.
                while chosen.len() < k {
                    chosen.push(pixels[0]);
                }
                break;
            }
        };

        use rand::distributions::Distribution;
        let next_idx = dist.sample(&mut rng);
        let next_centroid = pixels[next_idx];
        chosen.push(next_centroid);

        for (i, px) in pixels.iter().enumerate() {
            let dist_to_new = squared_distance(px, &next_centroid);
            if dist_to_new < min_sq_dists[i] {
                min_sq_dists[i] = dist_to_new;
            }
        }
    }

    chosen
}

/// Find the index of the centroid closest to `pixel` using squared Euclidean distance.
fn nearest_centroid(pixel: &Rgb, centroids: &[Rgb]) -> usize {
    centroids
        .iter()
        .enumerate()
        .min_by_key(|(_, c)| squared_distance(pixel, c))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Recompute each centroid as the mean of all pixels assigned to it.
fn recompute_centroids(pixels: &[Rgb], assignments: &[usize], k: usize) -> Vec<Rgb> {
    let mut sums = vec![(0u64, 0u64, 0u64, 0u64); k]; // (r, g, b, count)

    for (px, &cluster) in pixels.iter().zip(assignments.iter()) {
        let s = &mut sums[cluster];
        s.0 += px.r as u64;
        s.1 += px.g as u64;
        s.2 += px.b as u64;
        s.3 += 1;
    }

    sums.iter()
        .map(|(r, g, b, count)| {
            if *count == 0 {
                Rgb::new(0, 0, 0)
            } else {
                Rgb::new(
                    (r / count) as u8,
                    (g / count) as u8,
                    (b / count) as u8,
                )
            }
        })
        .collect()
}

/// Squared Euclidean distance between two colors (no sqrt needed for comparisons).
fn squared_distance(a: &Rgb, b: &Rgb) -> u32 {
    let dr = a.r as i32 - b.r as i32;
    let dg = a.g as i32 - b.g as i32;
    let db = a.b as i32 - b.b as i32;
    (dr * dr + dg * dg + db * db) as u32
}

/// Returns true if all centroids moved less than 2 units — good enough to stop.
fn centroids_converged(old: &[Rgb], new: &[Rgb]) -> bool {
    old.iter()
        .zip(new.iter())
        .all(|(a, b)| squared_distance(a, b) < 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(r: u8, g: u8, b: u8, n: usize) -> Vec<Rgb> {
        vec![Rgb::new(r, g, b); n]
    }

    // ── nearest_centroid ─────────────────────────────────────────────────────

    #[test]
    fn test_nearest_centroid_exact_match() {
        let centroids = vec![Rgb::new(255, 0, 0), Rgb::new(0, 255, 0)];
        assert_eq!(nearest_centroid(&Rgb::new(255, 0, 0), &centroids), 0);
        assert_eq!(nearest_centroid(&Rgb::new(0, 255, 0), &centroids), 1);
    }

    #[test]
    fn test_nearest_centroid_picks_closer_one() {
        let centroids = vec![Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)];
        // dark pixel should go to black centroid
        assert_eq!(nearest_centroid(&Rgb::new(10, 10, 10), &centroids), 0);
        // bright pixel should go to white centroid
        assert_eq!(nearest_centroid(&Rgb::new(240, 240, 240), &centroids), 1);
    }

    // ── squared_distance ─────────────────────────────────────────────────────

    #[test]
    fn test_squared_distance_same_color_is_zero() {
        let c = Rgb::new(100, 150, 200);
        assert_eq!(squared_distance(&c, &c), 0);
    }

    #[test]
    fn test_squared_distance_known_value() {
        let a = Rgb::new(0, 0, 0);
        let b = Rgb::new(1, 1, 1);
        assert_eq!(squared_distance(&a, &b), 3); // 1+1+1
    }

    // ── recompute_centroids ──────────────────────────────────────────────────

    #[test]
    fn test_recompute_centroids_single_cluster() {
        let pixels = vec![
            Rgb::new(100, 100, 100),
            Rgb::new(200, 200, 200),
        ];
        let assignments = vec![0, 0]; // both in cluster 0
        let result = recompute_centroids(&pixels, &assignments, 1);
        assert_eq!(result[0], Rgb::new(150, 150, 150));
    }

    #[test]
    fn test_recompute_centroids_two_clusters() {
        let pixels = vec![
            Rgb::new(0, 0, 0),
            Rgb::new(0, 0, 0),
            Rgb::new(255, 255, 255),
            Rgb::new(255, 255, 255),
        ];
        let assignments = vec![0, 0, 1, 1];
        let result = recompute_centroids(&pixels, &assignments, 2);
        assert_eq!(result[0], Rgb::new(0, 0, 0));
        assert_eq!(result[1], Rgb::new(255, 255, 255));
    }

    // ── convergence ──────────────────────────────────────────────────────────

    #[test]
    fn test_converged_when_identical() {
        let a = vec![Rgb::new(100, 100, 100)];
        assert!(centroids_converged(&a, &a));
    }

    #[test]
    fn test_not_converged_when_far_apart() {
        let a = vec![Rgb::new(0, 0, 0)];
        let b = vec![Rgb::new(255, 255, 255)];
        assert!(!centroids_converged(&a, &b));
    }

    // ── full generate ────────────────────────────────────────────────────────

    #[test]
    fn test_generate_empty_pixels_errors() {
        let result = KMeans.generate(&[], 8, 10);
        assert!(matches!(result, Err(RwalError::NoColorsExtracted)));
    }

    #[test]
    fn test_generate_solid_red_returns_red() {
        let pixels = solid(255, 0, 0, 500);
        let result = KMeans.generate(&pixels, 4, 10).unwrap();
        // all clusters should converge near red
        assert!(result.iter().all(|c| c.r > 200 && c.g < 50 && c.b < 50));
    }

    #[test]
    fn test_generate_returns_requested_count() {
        let pixels: Vec<Rgb> = (0..255).map(|i| Rgb::new(i, i, i)).collect();
        let result = KMeans.generate(&pixels, 8, 10).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_generate_fewer_pixels_than_k() {
        // 3 pixels, ask for 8 clusters — should not panic
        let pixels = vec![
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
        ];
        let result = KMeans.generate(&pixels, 8, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_two_distinct_clusters() {
        // 500 dark pixels + 500 bright pixels
        let mut pixels = solid(10, 10, 10, 500);
        pixels.extend(solid(245, 245, 245, 500));

        let result = KMeans.generate(&pixels, 2, 15).unwrap();
        assert_eq!(result.len(), 2);

        // one centroid should be dark, one bright
        let has_dark   = result.iter().any(|c| c.r < 100);
        let has_bright = result.iter().any(|c| c.r > 150);
        assert!(has_dark,   "expected a dark centroid");
        assert!(has_bright, "expected a bright centroid");
    }
}