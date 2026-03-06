use image::DynamicImage;
use palette::{FromColor, Lab, Srgb};
use rand::seq::SliceRandom;
use rayon::prelude::*;

const RESIZE_WIDTH: u32 = 256;

pub fn extract_colors(image: &DynamicImage, k: usize) -> Vec<Srgb<u8>> {
    let small_img = image.resize(RESIZE_WIDTH, RESIZE_WIDTH, image::imageops::FilterType::Triangle);
    let pixels: Vec<Srgb<u8>> = small_img
        .to_rgb8()
        .pixels()
        .map(|p| Srgb::new(p[0], p[1], p[2]))
        .collect();

    let lab_pixels: Vec<Lab> = pixels
        .par_iter()
        .map(|p| Lab::from_color(p.into_format()))
        .collect();

    let mut centroids: Vec<Lab> = lab_pixels
        .choose_multiple(&mut rand::thread_rng(), k)
        .cloned()
        .collect();

    for _ in 0..20 { // 20 iterations should be enough for convergence
        let mut new_centroids = vec![Lab::new(0.0, 0.0, 0.0); k];
        let mut counts = vec![0; k];
        
        for pixel in &lab_pixels {
            let mut min_dist = f32::MAX;
            let mut best_centroid = 0;

            for (i, centroid) in centroids.iter().enumerate() {
                let dist = squared_distance(pixel, centroid);
                if dist < min_dist {
                    min_dist = dist;
                    best_centroid = i;
                }
            }
            
            new_centroids[best_centroid].l += pixel.l;
            new_centroids[best_centroid].a += pixel.a;
            new_centroids[best_centroid].b += pixel.b;
            counts[best_centroid] += 1;
        }

        for i in 0..k {
            if counts[i] > 0 {
                new_centroids[i].l /= counts[i] as f32;
                new_centroids[i].a /= counts[i] as f32;
                new_centroids[i].b /= counts[i] as f32;
            }
        }
        
        if centroids.iter().zip(new_centroids.iter()).all(|(a, b)| squared_distance(a, b) < 1e-5) {
            break;
        }

        centroids = new_centroids;
    }

    centroids
        .into_iter()
        .map(|lab| Srgb::from_color(lab).into_format())
        .collect()
}

fn squared_distance(p1: &Lab, p2: &Lab) -> f32 {
    (p1.l - p2.l).powi(2) + (p1.a - p2.a).powi(2) + (p1.b - p2.b).powi(2)
}
