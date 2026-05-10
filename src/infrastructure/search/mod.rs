use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;

use crate::domain::models::reference::ReferenceEntry;
use crate::shared::error::AppError;

use super::resources::reference_reader::for_each_reference_entry;

const VECTOR_SCALE: f32 = 254.0;
const BUCKET_BINS: u16 = 8;
const BUCKET_COUNT: usize = 4096;
const TOP_K: usize = 5;
const MAGIC: &[u8; 8] = b"RB26IDX1";
const VERSION: u32 = 1;

#[derive(Debug)]
struct StoredReference {
    vector: [u8; 14],
    label: u8,
}

#[derive(Debug)]
pub struct LocalReferenceIndex {
    references: Vec<StoredReference>,
    buckets_a: Vec<Vec<u32>>,
    buckets_b: Vec<Vec<u32>>,
}

impl LocalReferenceIndex {
    pub fn load(index_path: &Path, references_path: &Path) -> Result<Self, AppError> {
        if index_path.exists() {
            return Self::from_compact_file(index_path);
        }
        Self::from_reference_file(references_path)
    }

    pub fn from_reference_file(path: &Path) -> Result<Self, AppError> {
        let file = File::open(path)?;
        let reader = BufReader::new(GzDecoder::new(file));
        let mut index = Self::new();

        for_each_reference_entry(reader, |entry| index.insert(entry))?;

        if index.references.is_empty() {
            return Err(AppError::Startup(
                "Reference dataset did not yield any vectors".into(),
            ));
        }

        Ok(index)
    }

    pub fn from_compact_file(path: &Path) -> Result<Self, AppError> {
        let mut reader = BufReader::new(File::open(path)?);

        let mut magic = [0_u8; 8];
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

        let count = read_u32(&mut reader)? as usize;
        let mut index = Self::with_capacity(count);

        for _ in 0..count {
            let label = read_u8(&mut reader)?;
            let mut vector = [0_u8; 14];
            reader.read_exact(&mut vector)?;
            index.push_quantized(vector, label)?;
        }

        Ok(index)
    }

    pub fn build_compact_file(reference_path: &Path, output_path: &Path) -> Result<(), AppError> {
        let input = File::open(reference_path)?;
        let reader = BufReader::new(GzDecoder::new(input));
        let temp_path = output_path.with_extension("tmp");
        let temp_output = File::create(&temp_path)?;
        let mut temp_writer = BufWriter::new(temp_output);
        let mut count = 0_u32;

        for_each_reference_entry(reader, |entry| {
            let (label, vector) = quantize_reference(entry)?;
            temp_writer.write_all(&[label])?;
            temp_writer.write_all(&vector)?;
            count += 1;
            Ok(())
        })?;

        temp_writer.flush()?;

        let output = File::create(output_path)?;
        let mut writer = BufWriter::new(output);
        writer.write_all(MAGIC)?;
        writer.write_all(&VERSION.to_le_bytes())?;
        writer.write_all(&count.to_le_bytes())?;

        let mut temp_reader = BufReader::new(File::open(&temp_path)?);
        std::io::copy(&mut temp_reader, &mut writer)?;
        writer.flush()?;
        std::fs::remove_file(temp_path)?;
        Ok(())
    }

    pub fn fraud_count_for(&self, query: &[f32; 14]) -> Result<usize, AppError> {
        let query_vector = quantize_vector(query);
        let candidates = self.candidate_ids(&query_vector);
        if candidates.is_empty() {
            return Err(AppError::Startup(
                "Reference index returned no candidates".into(),
            ));
        }

        let mut best_distances = [i64::MAX; TOP_K];
        let mut best_labels = [0_u8; TOP_K];

        for candidate_id in candidates {
            let reference = &self.references[candidate_id as usize];
            let distance = squared_distance(&query_vector, &reference.vector);
            if distance >= best_distances[TOP_K - 1] {
                continue;
            }

            let mut position = TOP_K - 1;
            while position > 0 && distance < best_distances[position - 1] {
                best_distances[position] = best_distances[position - 1];
                best_labels[position] = best_labels[position - 1];
                position -= 1;
            }

            best_distances[position] = distance;
            best_labels[position] = reference.label;
        }

        Ok(best_labels.iter().filter(|label| **label == 1).count())
    }

    fn new() -> Self {
        Self::with_capacity(0)
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            references: Vec::with_capacity(capacity),
            buckets_a: vec![Vec::new(); BUCKET_COUNT],
            buckets_b: vec![Vec::new(); BUCKET_COUNT],
        }
    }

    fn insert(&mut self, entry: ReferenceEntry) -> Result<(), AppError> {
        let (label, vector) = quantize_reference(entry)?;
        self.push_quantized(vector, label)
    }

    fn push_quantized(&mut self, vector: [u8; 14], label: u8) -> Result<(), AppError> {
        let id = self.references.len() as u32;
        self.references.push(StoredReference { vector, label });
        self.buckets_a[bucket_key_a_quantized(&vector) as usize].push(id);
        self.buckets_b[bucket_key_b_quantized(&vector) as usize].push(id);
        Ok(())
    }

    fn candidate_ids(&self, query: &[u8; 14]) -> Vec<u32> {
        let key_a = bucket_key_a_quantized(query);
        let key_b = bucket_key_b_quantized(query);
        let mut candidates = Vec::new();
        collect_bucket(&mut candidates, &self.buckets_a, key_a);
        collect_bucket(&mut candidates, &self.buckets_b, key_b);
        candidates.sort_unstable();
        candidates.dedup();

        if candidates.len() < 128 {
            for key in bucket_neighbors(key_a) {
                collect_bucket(&mut candidates, &self.buckets_a, key);
            }
            for key in bucket_neighbors(key_b) {
                collect_bucket(&mut candidates, &self.buckets_b, key);
            }
            candidates.sort_unstable();
            candidates.dedup();
        }

        candidates
    }
}

pub fn write_compact_index(reference_path: &Path, output_path: &Path) -> Result<(), AppError> {
    LocalReferenceIndex::build_compact_file(reference_path, output_path)
}

fn quantize_reference(entry: ReferenceEntry) -> Result<(u8, [u8; 14]), AppError> {
    let label = match entry.label.as_str() {
        "fraud" => 1,
        "legit" => 0,
        other => {
            return Err(AppError::Startup(format!(
                "Invalid label '{}' in reference dataset",
                other
            )));
        }
    };

    Ok((label, quantize_vector(&entry.vector)))
}

fn collect_bucket(target: &mut Vec<u32>, buckets: &[Vec<u32>], key: u16) {
    target.extend(buckets[key as usize].iter().copied());
}

fn quantize_vector(vector: &[f32; 14]) -> [u8; 14] {
    let mut out = [0_u8; 14];
    let mut index = 0;
    while index < 14 {
        out[index] = quantize_scalar(vector[index]);
        index += 1;
    }
    out
}

fn quantize_scalar(value: f32) -> u8 {
    if value.is_nan() {
        return 0;
    }
    if value < 0.0 {
        return 255;
    }
    (value.clamp(0.0, 1.0) * VECTOR_SCALE).round() as u8
}

fn squared_distance(lhs: &[u8; 14], rhs: &[u8; 14]) -> i64 {
    let mut total = 0_i64;
    let mut index = 0;
    while index < 14 {
        let delta = lhs[index] as i32 - rhs[index] as i32;
        total += (delta * delta) as i64;
        index += 1;
    }
    total
}

fn bucket_key_a_quantized(vector: &[u8; 14]) -> u16 {
    bucket_key_from_u8(vector, [0, 2, 7, 12])
}

fn bucket_key_b_quantized(vector: &[u8; 14]) -> u16 {
    bucket_key_from_u8(vector, [1, 3, 8, 13])
}

fn bucket_key_from_u8(vector: &[u8; 14], dimensions: [usize; 4]) -> u16 {
    let mut bins = [0_u16; 4];
    let mut index = 0;
    while index < 4 {
        bins[index] = bucket_bin_u8(vector[dimensions[index]]);
        index += 1;
    }
    pack_bins(bins)
}

fn bucket_bin_f32(value: f32) -> u16 {
    if value < 0.0 {
        return BUCKET_BINS - 1;
    }
    let clamped = value.clamp(0.0, 0.999_999_94);
    let bin = (clamped * BUCKET_BINS as f32).floor() as u16;
    bin.min(BUCKET_BINS - 1)
}

fn bucket_bin_u8(value: u8) -> u16 {
    if value == 255 {
        return BUCKET_BINS - 1;
    }
    bucket_bin_f32(value as f32 / VECTOR_SCALE)
}

fn pack_bins(bins: [u16; 4]) -> u16 {
    (bins[0] << 9) | (bins[1] << 6) | (bins[2] << 3) | bins[3]
}

fn unpack_bins(key: u16) -> [u16; 4] {
    [
        (key >> 9) & 0b111,
        (key >> 6) & 0b111,
        (key >> 3) & 0b111,
        key & 0b111,
    ]
}

fn bucket_neighbors(key: u16) -> Vec<u16> {
    let bins = unpack_bins(key);
    let mut variants = Vec::with_capacity(9);
    variants.push(key);

    let mut index = 0;
    while index < 4 {
        for delta in [-1_i16, 1_i16] {
            let candidate = bins[index] as i16 + delta;
            if (0..BUCKET_BINS as i16).contains(&candidate) {
                let mut next = bins;
                next[index] = candidate as u16;
                variants.push(pack_bins(next));
            }
        }
        index += 1;
    }

    variants.sort_unstable();
    variants.dedup();
    variants
}

fn read_u8(reader: &mut impl Read) -> Result<u8, AppError> {
    let mut buf = [0_u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32(reader: &mut impl Read) -> Result<u32, AppError> {
    let mut buf = [0_u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantizes_negative_sentinel() {
        assert_eq!(quantize_scalar(-1.0), 255);
    }

    #[test]
    fn packs_and_unpacks_bucket_bins() {
        let key = pack_bins([1, 2, 3, 4]);
        assert_eq!(unpack_bins(key), [1, 2, 3, 4]);
    }
}
