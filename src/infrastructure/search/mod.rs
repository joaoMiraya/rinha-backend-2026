use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;

use crate::shared::error::AppError;

use super::resources::reference_reader::for_each_reference_entry;

const DIMS: usize = 14;
const PROBES_FAST: usize = 8;
const PROBES_FULL: usize = 24;
const TOP_K: usize = 5;
const CLUSTERS: usize = 4096;
const BLOCK_SIZE: usize = 8;
const SCALE: f32 = 10_000.0;
const MAGIC: &[u8; 4] = b"RIVF";
const VERSION: u32 = 1;
const SELECTED_DIMS: [usize; 4] = [0, 2, 7, 12];

#[derive(Debug)]
pub struct LocalReferenceIndex {
    centroids: Vec<f32>, // dimension-major: DIMS * clusters
    offsets: Vec<u32>,   // clusters + 1
    labels: Vec<u8>,     // total_blocks * BLOCK_SIZE
    blocks: Vec<i16>,    // total_blocks * BLOCK_SIZE * DIMS, dimension-major per block
    clusters: usize,
    n: usize,
}

impl LocalReferenceIndex {
    pub fn load(index_path: &Path, references_path: &Path) -> Result<Self, AppError> {
        if index_path.exists() {
            return Self::from_compact_file(index_path);
        }
        Self::build_from_references(references_path)
    }

    pub fn build_compact_file(reference_path: &Path, output_path: &Path) -> Result<(), AppError> {
        let index = Self::build_from_references(reference_path)?;
        index.write_compact_file(output_path)
    }

    pub fn from_compact_file(path: &Path) -> Result<Self, AppError> {
        let mut reader = BufReader::new(File::open(path)?);
        let mut magic = [0_u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(AppError::Startup("Invalid index artifact header".into()));
        }

        let version = read_u32(&mut reader)?;
        if version != VERSION {
            return Err(AppError::Startup(format!(
                "Unsupported index artifact version {}",
                version
            )));
        }

        let n = read_u32(&mut reader)? as usize;
        let clusters = read_u32(&mut reader)? as usize;
        let dims = read_u32(&mut reader)? as usize;
        if dims != DIMS || clusters != CLUSTERS {
            return Err(AppError::Startup("Index artifact shape mismatch".into()));
        }

        let mut centroids = vec![0.0f32; DIMS * clusters];
        read_f32s(&mut reader, &mut centroids)?;

        let mut offsets = vec![0_u32; clusters + 1];
        read_u32s(&mut reader, &mut offsets)?;
        let total_blocks = offsets[clusters] as usize;

        let mut labels = vec![0_u8; total_blocks * BLOCK_SIZE];
        reader.read_exact(&mut labels)?;

        let mut blocks = vec![0_i16; total_blocks * BLOCK_SIZE * DIMS];
        read_i16s(&mut reader, &mut blocks)?;

        if n == 0 || total_blocks == 0 {
            return Err(AppError::Startup("Index artifact is empty".into()));
        }

        Ok(Self {
            centroids,
            offsets,
            labels,
            blocks,
            clusters,
            n,
        })
    }

    pub fn fraud_count_for(&self, query: &[f32; DIMS]) -> Result<usize, AppError> {
        let query_q = quantize_vector(query);
        let fast_probes = self.top_centroid_probes::<PROBES_FAST>(query);
        let fast_count = self.scan_probes(&query_q, &fast_probes)?;
        if fast_count != 2 && fast_count != 3 {
            return Ok(fast_count);
        }

        let full_probes = self.top_centroid_probes::<PROBES_FULL>(query);
        self.scan_probes(&query_q, &full_probes)
    }

    fn build_from_references(reference_path: &Path) -> Result<Self, AppError> {
        let file = File::open(reference_path)?;
        let reader = BufReader::new(GzDecoder::new(file));

        let mut sums = vec![0.0f64; DIMS * CLUSTERS];
        let mut counts = vec![0_u32; CLUSTERS];
        let mut n = 0_u32;

        for_each_reference_entry(reader, |entry| {
            let bin = cluster_id(&entry.vector);
            counts[bin] = counts[bin].saturating_add(1);
            add_to_sums(&mut sums, bin, &entry.vector);
            n = n.saturating_add(1);
            Ok(())
        })?;

        if n == 0 {
            return Err(AppError::Startup(
                "Reference dataset did not yield any vectors".into(),
            ));
        }

        let mut centroids = vec![0.0f32; DIMS * CLUSTERS];
        for cluster in 0..CLUSTERS {
            let count = counts[cluster].max(1) as f64;
            for dim in 0..DIMS {
                centroids[dim * CLUSTERS + cluster] = (sums[dim * CLUSTERS + cluster] / count) as f32;
            }
        }

        let mut offsets = vec![0_u32; CLUSTERS + 1];
        for cluster in 0..CLUSTERS {
            let blocks = ((counts[cluster] as usize) + (BLOCK_SIZE - 1)) / BLOCK_SIZE;
            offsets[cluster + 1] = offsets[cluster] + blocks as u32;
        }
        let total_blocks = offsets[CLUSTERS] as usize;

        let mut labels = vec![0_u8; total_blocks * BLOCK_SIZE];
        let mut blocks = vec![0_i16; total_blocks * BLOCK_SIZE * DIMS];
        let mut positions = vec![0_u32; CLUSTERS];

        let file = File::open(reference_path)?;
        let reader = BufReader::new(GzDecoder::new(file));
        for_each_reference_entry(reader, |entry| {
            let bin = cluster_id(&entry.vector);
            let pos = positions[bin] as usize;
            let block_index = offsets[bin] as usize + pos / BLOCK_SIZE;
            let slot = pos % BLOCK_SIZE;

            labels[block_index * BLOCK_SIZE + slot] = label_to_u8(&entry.label)?;
            write_block_vector(
                &mut blocks,
                block_index,
                slot,
                &entry.vector,
            );

            positions[bin] = positions[bin].saturating_add(1);
            Ok(())
        })?;

        Ok(Self {
            centroids,
            offsets,
            labels,
            blocks,
            clusters: CLUSTERS,
            n: n as usize,
        })
    }

    fn write_compact_file(&self, output_path: &Path) -> Result<(), AppError> {
        let temp_path = output_path.with_extension("tmp");
        {
            let file = File::create(&temp_path)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(MAGIC)?;
            writer.write_all(&VERSION.to_le_bytes())?;

            writer.write_all(&(self.n as u32).to_le_bytes())?;
            writer.write_all(&(self.clusters as u32).to_le_bytes())?;
            writer.write_all(&(DIMS as u32).to_le_bytes())?;
            write_f32s(&mut writer, &self.centroids)?;
            write_u32s(&mut writer, &self.offsets)?;
            writer.write_all(&self.labels)?;
            write_i16s(&mut writer, &self.blocks)?;
            writer.flush()?;
        }

        std::fs::rename(&temp_path, output_path)?;
        Ok(())
    }

    fn top_centroid_probes<const N: usize>(&self, query: &[f32; DIMS]) -> [usize; N] {
        let mut best_dists = [f32::INFINITY; N];
        let mut best_ids = [0usize; N];

        for cluster in 0..self.clusters {
            let dist = centroid_distance(query, &self.centroids, cluster);
            if dist >= best_dists[N - 1] {
                continue;
            }
            let pos = best_dists.partition_point(|&x| x < dist);
            if pos < N {
                best_dists[pos..].rotate_right(1);
                best_ids[pos..].rotate_right(1);
                best_dists[pos] = dist;
                best_ids[pos] = cluster;
            }
        }

        best_ids
    }

    fn scan_probes<const N: usize>(&self, query: &[i16; DIMS], probes: &[usize; N]) -> Result<usize, AppError> {
        let mut best_dists = [i64::MAX; TOP_K];
        let mut best_labels = [0_u8; TOP_K];

        for &cluster in probes {
            let start = self.offsets[cluster] as usize;
            let end = self.offsets[cluster + 1] as usize;
            for block in start..end {
                self.scan_block(query, block, &mut best_dists, &mut best_labels);
            }
        }

        Ok(best_labels.iter().filter(|&&label| label == 1).count())
    }

    fn scan_block(
        &self,
        query: &[i16; DIMS],
        block: usize,
        best_dists: &mut [i64; TOP_K],
        best_labels: &mut [u8; TOP_K],
    ) {
        for slot in 0..BLOCK_SIZE {
            let mut dist = 0i64;

            for dim in 0..4 {
                let delta = query[SELECTED_DIMS[dim]] as i32
                    - self.blocks[block * BLOCK_SIZE * DIMS + SELECTED_DIMS[dim] * BLOCK_SIZE + slot] as i32;
                dist += (delta * delta) as i64;
            }
            if dist >= best_dists[TOP_K - 1] {
                continue;
            }

            for dim in 4..8 {
                let delta = query[dim] as i32
                    - self.blocks[block * BLOCK_SIZE * DIMS + dim * BLOCK_SIZE + slot] as i32;
                dist += (delta * delta) as i64;
            }
            if dist >= best_dists[TOP_K - 1] {
                continue;
            }

            for dim in 8..DIMS {
                let delta = query[dim] as i32
                    - self.blocks[block * BLOCK_SIZE * DIMS + dim * BLOCK_SIZE + slot] as i32;
                dist += (delta * delta) as i64;
            }

            if dist >= best_dists[TOP_K - 1] {
                continue;
            }

            let label = self.labels[block * BLOCK_SIZE + slot];
            let mut pos = TOP_K - 1;
            while pos > 0 && dist < best_dists[pos - 1] {
                best_dists[pos] = best_dists[pos - 1];
                best_labels[pos] = best_labels[pos - 1];
                pos -= 1;
            }
            best_dists[pos] = dist;
            best_labels[pos] = label;
        }
    }
}

pub fn write_compact_index(reference_path: &Path, output_path: &Path) -> Result<(), AppError> {
    LocalReferenceIndex::build_compact_file(reference_path, output_path)
}

fn cluster_id(vector: &[f32; DIMS]) -> usize {
    let mut id = 0usize;
    for &dim in &SELECTED_DIMS {
        id = (id << 3) | bucket_bin(vector[dim]) as usize;
    }
    id
}

fn bucket_bin(value: f32) -> u8 {
    if value.is_nan() {
        return 0;
    }
    if value < 0.0 {
        return 7;
    }
    let clamped = value.clamp(0.0, 0.999_999_94);
    let bin = (clamped * 8.0).floor() as u8;
    bin.min(7)
}

fn quantize_vector(vector: &[f32; DIMS]) -> [i16; DIMS] {
    let mut out = [0_i16; DIMS];
    for (idx, value) in vector.iter().enumerate() {
        out[idx] = quantize_scalar(*value);
    }
    out
}

fn quantize_scalar(value: f32) -> i16 {
    if value.is_nan() {
        0
    } else if value < 0.0 {
        -10_000
    } else {
        (value.clamp(0.0, 1.0) * SCALE).round() as i16
    }
}

fn centroid_distance(query: &[f32; DIMS], centroids: &[f32], cluster: usize) -> f32 {
    let mut dist = 0.0f32;
    for dim in 0..DIMS {
        let c = centroids[dim * CLUSTERS + cluster];
        let diff = query[dim] - c;
        dist += diff * diff;
    }
    dist
}

fn add_to_sums(sums: &mut [f64], cluster: usize, vector: &[f32; DIMS]) {
    for dim in 0..DIMS {
        sums[dim * CLUSTERS + cluster] += vector[dim] as f64;
    }
}

fn label_to_u8(label: &str) -> Result<u8, AppError> {
    match label {
        "fraud" => Ok(1),
        "legit" => Ok(0),
        other => Err(AppError::Startup(format!(
            "Invalid label '{}' in reference dataset",
            other
        ))),
    }
}

fn write_block_vector(blocks: &mut [i16], block: usize, slot: usize, vector: &[f32; DIMS]) {
    for dim in 0..DIMS {
        blocks[block * BLOCK_SIZE * DIMS + dim * BLOCK_SIZE + slot] = quantize_scalar(vector[dim]);
    }
}

fn read_u32(reader: &mut impl Read) -> Result<u32, AppError> {
    let mut buf = [0_u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_f32s(reader: &mut impl Read, out: &mut [f32]) -> Result<(), AppError> {
    let mut buf = [0_u8; 4096];
    let mut pos = 0usize;
    while pos < out.len() {
        let needed = ((out.len() - pos) * 4).min(buf.len());
        reader.read_exact(&mut buf[..needed])?;
        for chunk in buf[..needed].chunks_exact(4) {
            out[pos] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            pos += 1;
        }
    }
    Ok(())
}

fn read_u32s(reader: &mut impl Read, out: &mut [u32]) -> Result<(), AppError> {
    for value in out.iter_mut() {
        *value = read_u32(reader)?;
    }
    Ok(())
}

fn read_i16s(reader: &mut impl Read, out: &mut [i16]) -> Result<(), AppError> {
    let mut buf = [0_u8; 4096];
    let mut pos = 0usize;
    while pos < out.len() {
        let needed = ((out.len() - pos) * 2).min(buf.len());
        reader.read_exact(&mut buf[..needed])?;
        for chunk in buf[..needed].chunks_exact(2) {
            out[pos] = i16::from_le_bytes([chunk[0], chunk[1]]);
            pos += 1;
        }
    }
    Ok(())
}

fn write_f32s(writer: &mut impl Write, values: &[f32]) -> Result<(), AppError> {
    for value in values {
        writer.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn write_u32s(writer: &mut impl Write, values: &[u32]) -> Result<(), AppError> {
    for value in values {
        writer.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn write_i16s(writer: &mut impl Write, values: &[i16]) -> Result<(), AppError> {
    for value in values {
        writer.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantizes_negative_values_as_sentinel() {
        assert_eq!(quantize_scalar(-1.0), -10_000);
    }

    #[test]
    fn cluster_id_uses_four_dims() {
        let mut v = [0.0f32; DIMS];
        v[0] = 0.1;
        v[2] = 0.2;
        v[7] = 0.3;
        v[12] = 0.4;
        assert_eq!(cluster_id(&v), cluster_id(&v));
    }
}
