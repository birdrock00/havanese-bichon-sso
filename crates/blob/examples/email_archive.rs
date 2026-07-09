/// bichon-blob usage example: email archival with content-addressable storage.
///
/// This example simulates a mail archival system where multiple accounts
/// may receive the same email (e.g. CC'd or forwarded).  The blob store
/// deduplicates by content hash, and the upper application layer tracks
/// which accounts reference each hash.
///
/// Run: cargo run --example email_archive

use std::collections::HashMap;

use bichon_blob::{Codec, Config, Engine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Setup ──────────────────────────────────────────────────────────

    let store_path = std::path::Path::new("target/example_blob_store");
    let _ = std::fs::remove_dir_all(store_path); // clean up from previous run
    let engine = Engine::open(store_path, Config::default())?;

    println!("Store opened at: {:?}\n", store_path);

    // ── Simulated upper-layer reference tracker ────────────────────────
    let mut refs: HashMap<String, Vec<[u8; 32]>> = HashMap::new();

    // ── 1. Store emails ────────────────────────────────────────────────

    // Simulate three emails arriving.  Email #2 is a newsletter that
    // both alice and bob received — identical content, same hash.
    let emails = vec![
        ("alice", "Welcome to Bichon Mail!"),
        ("alice", "Weekly Newsletter: Rust Edition"),
        ("bob", "Weekly Newsletter: Rust Edition"), // same content as above
    ];

    for (account, body) in &emails {
        let hash = mock_content_hash(body.as_bytes());
        let account_refs = refs.entry(account.to_string()).or_default();

        // Check if this content already exists in the blob store
        if engine.exists(&hash)? {
            println!(
                "[dedup] {}: hash {:02x?}... already stored, skipping",
                account,
                &hash[..4]
            );
        } else {
            engine.put(hash, body.as_bytes(), Codec::Zstd)?;
            println!(
                "[store] {}: hash {:02x?}..., {} bytes",
                account,
                &hash[..4],
                body.len()
            );
        }

        account_refs.push(hash);
    }

    println!();

    // ── 2. Read back emails ────────────────────────────────────────────

    let hash = mock_content_hash(b"Weekly Newsletter: Rust Edition");
    let stored = engine.get(&hash)?;
    println!(
        "Read newsletter: {:?}",
        stored.map(|v| String::from_utf8_lossy(&v).to_string())
    );

    // ── 3. Batch store attachments ─────────────────────────────────────

    let attachments: Vec<([u8; 32], Vec<u8>, Codec)> = (0..10)
        .map(|i| {
            let body = format!("Attachment #{}: {}", i, "X".repeat(5000));
            let hash = mock_content_hash(body.as_bytes());
            (hash, body.into_bytes(), Codec::Zstd)
        })
        .collect();

    engine.put_batch(&attachments)?;
    println!("\nBatch-stored {} attachments", attachments.len());

    // ── 4. Stats ───────────────────────────────────────────────────────

    let stats = engine.stats()?;
    println!(
        "Stats: {} keys, {} bytes, {} segments",
        stats.total_keys, stats.total_bytes, stats.segment_count
    );

    // ── 5. Delete an email (simulating: last reference removed) ─────────

    // In production, before calling engine.delete(), you'd check:
    //   SELECT COUNT(*) FROM email_refs WHERE content_hash = ? AND account_id != ?
    // If count == 0, it's safe to delete from blob.

    let welcome_hash = mock_content_hash(b"Welcome to Bichon Mail!");
    engine.delete(&welcome_hash)?;
    println!("\nDeleted welcome email, still exists? {}", engine.exists(&welcome_hash)?);

    // ── 6. Shutdown ────────────────────────────────────────────────────

    engine.shutdown()?;
    println!("\nClean shutdown complete.");

    Ok(())
}

/// In production, use BLAKE3 or SHA-256.
/// Here we use a trivial hash for demonstration.
fn mock_content_hash(data: &[u8]) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let h = hasher.finish();

    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&h.to_le_bytes());
    // Mix in the length so different-sized content gets different hashes
    key[8..16].copy_from_slice(&(data.len() as u64).to_le_bytes());
    key
}
