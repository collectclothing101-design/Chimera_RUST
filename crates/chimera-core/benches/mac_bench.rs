// chimera-core/benches/mac_bench.rs
// Performance benchmarks for MAC address validation and operations.

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_validate_mac_colon(c: &mut Criterion) {
    c.bench_function("validate_mac_colon", |b| {
        b.iter(|| {
            chimera_core::mac_address::validate_mac("AA:BB:CC:DD:EE:FF").unwrap();
        });
    });
}

fn bench_validate_mac_hyphen(c: &mut Criterion) {
    c.bench_function("validate_mac_hyphen", |b| {
        b.iter(|| {
            chimera_core::mac_address::validate_mac("AA-BB-CC-DD-EE-FF").unwrap();
        });
    });
}

fn bench_validate_mac_invalid(c: &mut Criterion) {
    c.bench_function("validate_mac_invalid", |b| {
        b.iter(|| {
            chimera_core::mac_address::validate_mac("invalid").ok();
        });
    });
}

fn bench_format_mac(c: &mut Criterion) {
    let bytes = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    c.bench_function("format_mac", |b| {
        b.iter(|| {
            chimera_core::mac_address::format_mac(&bytes);
        });
    });
}

fn bench_is_locally_administered(c: &mut Criterion) {
    let bytes = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00];
    c.bench_function("is_locally_administered", |b| {
        b.iter(|| {
            chimera_core::mac_address::is_locally_administered(&bytes);
        });
    });
}

fn bench_is_multicast(c: &mut Criterion) {
    let bytes = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
    c.bench_function("is_multicast", |b| {
        b.iter(|| {
            chimera_core::mac_address::is_multicast(&bytes);
        });
    });
}

fn bench_derive_mac(c: &mut Criterion) {
    c.bench_function("derive_mac_from_seed", |b| {
        b.iter(|| {
            chimera_core::mac_address::derive_mac_from_seed("test", 0);
        });
    });
}

fn bench_pack_samsung_nv_mac(c: &mut Criterion) {
    let bytes = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    c.bench_function("pack_samsung_nv_mac", |b| {
        b.iter(|| {
            chimera_core::mac_address::pack_samsung_nv_mac(&bytes);
        });
    });
}

fn bench_unpack_samsung_nv_mac(c: &mut Criterion) {
    let packed = chimera_core::mac_address::pack_samsung_nv_mac(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    c.bench_function("unpack_samsung_nv_mac", |b| {
        b.iter(|| {
            chimera_core::mac_address::unpack_samsung_nv_mac(&packed);
        });
    });
}

criterion_group!(
    benches,
    bench_validate_mac_colon,
    bench_validate_mac_hyphen,
    bench_validate_mac_invalid,
    bench_format_mac,
    bench_is_locally_administered,
    bench_is_multicast,
    bench_derive_mac,
    bench_pack_samsung_nv_mac,
    bench_unpack_samsung_nv_mac,
);

criterion_main!(benches);
