use std::{iter::repeat_with, time::Instant};

use rand::Rng;
use reed_solomon_simd::{ReedSolomonDecoder, ReedSolomonEncoder};

// Function to generate random data of a given size
fn generate_random_data(size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    repeat_with(|| rng.gen()).take(size).collect()
}

// Function to calculate the padded size
fn calculate_padded_size(size: usize, num_original_shards: usize) -> usize {
    let base_shard_size = (size + num_original_shards - 1) / num_original_shards;
    let shard_size = ((base_shard_size + 63) / 64) * 64; // Round up to nearest multiple of 64
    shard_size * num_original_shards
}

fn encode_decode_benchmark() {
    let sizes = [
        //("1KB", 1024),
        ("1MB", 1024 * 1024),
        ("1GB", 1024 * 1024 * 1024),
        ("3GB", 2 * 1024 * 1024 * 1024),
    ];

    let shard_configs = [
        (21, 10),   // 21 original, 10 recovery
        (67, 33),   // 67 original, 33 recovery
        (201, 100), // 201 original, 100 recovery
        (667, 333), // 667 original, 333 recovery
    ];

    // Determine the maximum padded size needed
    let max_padded_size = sizes
        .iter()
        .map(|&(_, size)| {
            shard_configs
                .iter()
                .map(move |&(num_original_shards, _)| {
                    calculate_padded_size(size, num_original_shards)
                })
                .max()
                .unwrap()
        })
        .max()
        .unwrap();

    // Generate the maximum data size once, including padding
    let max_data = generate_random_data(max_padded_size);

    for &(label, size) in &sizes {
        for &(num_original_shards, num_recovery_shards) in &shard_configs {
            // Calculate the padded size
            let padded_size = calculate_padded_size(size, num_original_shards);

            let data = &max_data[..padded_size]; // Use a slice of the pre-generated data
            let original: Vec<&[u8]> = data.chunks(padded_size / num_original_shards).collect();

            if original.len() != num_original_shards {
                println!(
                    "Data size too small to divide into {} shards for {}",
                    num_original_shards, label
                );
                continue;
            }

            // Encoding
            let mut encoder = ReedSolomonEncoder::new(
                num_original_shards,               // total number of original shards
                num_recovery_shards,               // total number of recovery shards
                padded_size / num_original_shards, // shard size in bytes
            )
            .unwrap();

            for shard in &original {
                encoder.add_original_shard(shard).unwrap();
            }

            let start_encode = Instant::now();
            let result = encoder.encode().unwrap();
            let recovery: Vec<_> = result.recovery_iter().collect();
            let duration_encode = start_encode.elapsed();
            let throughput_encode = (size as f64) / duration_encode.as_secs_f64() / 1_073_741_824.0;
            println!("Encoding {} with {} original and {} recovery shards took: {:?}, throughput: {:.2} GiB/sec", label, num_original_shards, num_recovery_shards, duration_encode, throughput_encode);

            // Decoding
            let mut decoder = ReedSolomonDecoder::new(
                num_original_shards,               // total number of original shards
                num_recovery_shards,               // total number of recovery shards
                padded_size / num_original_shards, // shard size in bytes
            )
            .unwrap();

            // Add some original and all recovery shards to the decoder
            for (i, shard) in original.iter().enumerate().skip(num_recovery_shards) {
                decoder.add_original_shard(i, shard).unwrap();
            }
            for (i, shard) in recovery.iter().enumerate() {
                decoder.add_recovery_shard(i, shard).unwrap();
            }

            let start_decode = Instant::now();
            decoder.decode().unwrap();
            let duration_decode = start_decode.elapsed();
            let throughput_decode = (size as f64) / duration_decode.as_secs_f64() / 1_073_741_824.0;
            println!("Decoding {} with {} original and {} recovery shards took: {:?}, throughput: {:.2} GiB/sec", label, num_original_shards, num_recovery_shards, duration_decode, throughput_decode);
        }
    }
}

fn main() {
    encode_decode_benchmark();
}
