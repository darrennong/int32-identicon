//! Find the two multicolored arcs of the fixed frame without relying on a
//! globally white background. The monochrome payload and ordinary gray text
//! cannot by themselves satisfy the hue-diversity gate. All geometry remains
//! a proposal: scan.rs still validates the full frame, payload, RS and CRC.
use crate::scan::{Point, Quad};
use crate::Error;

fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn length(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn hull(mut points: Vec<Point>) -> Vec<Point> {
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let mut result: Vec<Point> = Vec::new();
    for &p in &points {
        while result.len() > 1
            && cross(result[result.len() - 2], result[result.len() - 1], p) <= 0.0
        {
            result.pop();
        }
        result.push(p);
    }
    let lower = result.len();
    for &p in points.iter().rev().skip(1) {
        while result.len() > lower
            && cross(result[result.len() - 2], result[result.len() - 1], p) <= 0.0
        {
            result.pop();
        }
        result.push(p);
    }
    result.pop();
    result
}

fn simplify(mut polygon: Vec<Point>, epsilon: f64) -> Vec<Point> {
    while polygon.len() > 4 {
        let n = polygon.len();
        let (index, error) = (0..n)
            .map(|i| {
                let a = polygon[(i + n - 1) % n];
                let b = polygon[(i + 1) % n];
                (i, cross(a, b, polygon[i]).abs() / length(a, b).max(1e-9))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        if error > epsilon {
            break;
        }
        polygon.remove(index);
    }
    polygon
}

fn intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let cd = [d[0] - c[0], d[1] - c[1]];
    let denom = ab[0] * cd[1] - ab[1] * cd[0];
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = ((c[0] - a[0]) * cd[1] - (c[1] - a[1]) * cd[0]) / denom;
    Some([a[0] + t * ab[0], a[1] + t * ab[1]])
}

fn frame_quad(points: Vec<Point>, width: usize, height: usize, step: usize) -> Option<Quad> {
    let polygon = hull(points);
    if polygon.len() < 4 {
        return None;
    }
    let min_x = polygon.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let max_x = polygon.iter().map(|p| p[0]).fold(0.0, f64::max);
    let min_y = polygon.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let max_y = polygon.iter().map(|p| p[1]).fold(0.0, f64::max);
    if (max_x - min_x).min(max_y - min_y) < 24.0 {
        return None;
    }
    let epsilon = ((max_x - min_x).hypot(max_y - min_y) * 0.004).max(step as f64 * 1.25);
    let polygon = simplify(polygon, epsilon);
    let n = polygon.len();
    // The missing neutral-color corner patches create two short diagonal
    // chamfers. Extend the four long sides, rather than treating those cut
    // corners as the actual corners of the symbol.
    let mut edges: Vec<usize> = (0..n).collect();
    edges.sort_by(|&a, &b| {
        length(polygon[b], polygon[(b + 1) % n])
            .total_cmp(&length(polygon[a], polygon[(a + 1) % n]))
    });
    let longest = length(polygon[edges[0]], polygon[(edges[0] + 1) % n]);
    if length(polygon[edges[3]], polygon[(edges[3] + 1) % n]) < longest * 0.38 {
        return None;
    }
    edges.truncate(4);
    edges.sort_unstable();
    let mut q = [[0.0; 2]; 4];
    for i in 0..4 {
        let a = edges[(i + 3) % 4];
        let b = edges[i];
        q[i] = intersection(
            polygon[a],
            polygon[(a + 1) % n],
            polygon[b],
            polygon[(b + 1) % n],
        )?;
        if q[i][0] < -(step as f64 * 2.0)
            || q[i][1] < -(step as f64 * 2.0)
            || q[i][0] > width as f64 + step as f64 * 2.0
            || q[i][1] > height as f64 + step as f64 * 2.0
        {
            return None;
        }
        q[i][0] = q[i][0].clamp(0.0, (width - 1) as f64);
        q[i][1] = q[i][1].clamp(0.0, (height - 1) as f64);
    }
    let mut min_edge = f64::INFINITY;
    let mut max_edge = 0.0_f64;
    let mut area = 0.0;
    for i in 0..4 {
        if cross(q[i], q[(i + 1) % 4], q[(i + 2) % 4]) <= 0.0 {
            return None;
        }
        let side = length(q[i], q[(i + 1) % 4]);
        min_edge = min_edge.min(side);
        max_edge = max_edge.max(side);
        area += q[i][0] * q[(i + 1) % 4][1] - q[(i + 1) % 4][0] * q[i][1];
    }
    if min_edge < 24.0 || max_edge > min_edge * 3.0 || area < 1024.0 {
        return None;
    }
    let first = (0..4)
        .min_by(|&i, &j| (q[i][0] + q[i][1]).total_cmp(&(q[j][0] + q[j][1])))
        .unwrap();
    Some(std::array::from_fn(|i| q[(first + i) % 4]))
}

fn hue(pixel: &[u8]) -> u8 {
    if pixel[3] < 128 {
        return 0;
    }
    let [r, g, b] = [pixel[0] as f64, pixel[1] as f64, pixel[2] as f64];
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    if max < 55.0 || d < 28.0 || d < max * 0.24 {
        return 0;
    }
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h.floor() as u8).min(5) + 1
}

pub(crate) fn candidates(width: usize, height: usize, pixels: &[u8]) -> Result<Vec<Quad>, Error> {
    let step = width.max(height).div_ceil(960);
    let w = width.div_ceil(step);
    let h = height.div_ceil(step);
    let mut mask = vec![0; w * h];
    for y in 0..h {
        for x in 0..w {
            let sx = (x * step + step / 2).min(width - 1);
            let sy = (y * step + step / 2).min(height - 1);
            mask[y * w + x] = hue(&pixels[(sy * width + sx) * 4..(sy * width + sx) * 4 + 4]);
        }
    }
    // Adjacent complementary colors can blend to a gray seam after camera
    // sampling/JPEG. Bridge at most one sample on either side for connectivity
    // only; retain original chromatic pixels for hull geometry and hue counts.
    let mut connected = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            if mask[y * w + x] != 0 {
                for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                        connected[ny * w + nx] = true;
                    }
                }
            }
        }
    }
    let mut visited = vec![false; w * h];
    let mut queue = Vec::new();
    let mut components = Vec::new();
    for start in 0..mask.len() {
        if !connected[start] || visited[start] {
            continue;
        }
        queue.clear();
        queue.push(start);
        visited[start] = true;
        let mut count = [0_usize; 6];
        let mut boundary = Vec::new();
        let mut head = 0;
        while head < queue.len() {
            let index = queue[head];
            head += 1;
            let y = index / w;
            let x = index % w;
            if mask[index] != 0 {
                count[(mask[index] - 1) as usize] += 1;
            }
            let mut outer = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let next = ny * w + nx;
                    if mask[next] == 0 {
                        outer = true;
                    }
                    if connected[next] && !visited[next] {
                        visited[next] = true;
                        queue.push(next);
                    }
                }
            }
            if outer && mask[index] != 0 {
                boundary.push([(x * step + step / 2) as f64, (y * step + step / 2) as f64]);
            }
        }
        let colored: usize = count.iter().sum();
        let threshold = (colored / 50).max(4);
        if colored >= 32 && count.iter().filter(|&&n| n >= threshold).count() >= 4 {
            components.push(hull(boundary));
            // Never silently drop additional frame candidates and then report
            // a first hit. Busy scenes exceed this prototype's bounded budget.
            if components.len() > 12 {
                return Err(Error::Quality);
            }
        }
    }
    let mut result = Vec::new();
    for (i, a) in components.iter().enumerate() {
        for b in components.iter().skip(i) {
            let points = a.iter().chain(b).copied().collect();
            if let Some(q) = frame_quad(points, width, height, step) {
                if !result
                    .iter()
                    .any(|old: &Quad| old.iter().zip(q).all(|(&x, y)| length(x, y) < 2.0))
                {
                    result.push(q);
                }
            }
        }
    }
    Ok(result)
}
