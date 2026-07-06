// chimera-core/benches/imei_bench.rs
// Performance benchmarks for IMEI validation and operations.

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_validate_imei_valid(c: &mut Criterion) {
    c.bench_function("validate_imei_valid", |b| {
        b.iter(|| {
            chimera_core::imei::validate_imei("352099001761481").unwrap();
        });
    });
}

fn bench_validate_imei_invalid(c: &mut Criterion) {
    c.bench_function("validate_imei_invalid", |b| {
        b.iter(|| {
            chimera_core::imei::validate_imei("12345").ok();
        });
    });
}

fn bench_calculate_check_digit(c: &mut Criterion) {
    c.bench_function("calculate_check_digit", |b| {
        b.iter(|| {
            chimera_core::imei::calculate_check_digit("35209900176148").unwrap();
        });
    });
}

fn bench_complete_imei(c: &mut Criterion) {
    c.bench_function("complete_imei", |b| {
        b.iter(|| {
            chimera_core::imei::complete_imei("35209900176148").unwrap();
        });
    });
}

fn bench_format_imei(c: &mut Criterion) {
    c.bench_function("format_imei", |b| {
        b.iter(|| {
            chimera_core::imei::format_imei("352099001761481");
        });
    });
}

fn bench_get_tac(c: &mut Criterion) {
    c.bench_function("get_tac", |b| {
        b.iter(|| {
            chimera_core::imei::get_tac("352099001761481");
        });
    });
}

fn bench_imei_to_bytes(c: &mut Criterion) {
    c.bench_function("imei_to_bytes", |b| {
        b.iter(|| {
            chimera_core::imei::imei_to_bytes("352099001761481");
        });
    });
}

fn bench_bytes_to_imei(c: &mut Criterion) {
    let bytes = chimera_core::imei::imei_to_bytes("352099001761481");
    c.bench_function("bytes_to_imei", |b| {
        b.iter(|| {
            chimera_core::imei::bytes_to_imei(&bytes);
        });
    });
}

fn bench_calculate_network_code(c: &mut Criterion) {
    c.bench_function("calculate_network_code", |b| {
        b.iter(|| {
            chimera_core::imei::calculate_network_code("352099001761481");
        });
    });
}

criterion_group!(
    benches,
    bench_validate_imei_valid,
    bench_validate_imei_invalid,
    bench_calculate_check_digit,
    bench_complete_imei,
    bench_format_imei,
    bench_get_tac,
    bench_imei_to_bytes,
    bench_bytes_to_imei,
    bench_calculate_network_code,
);

criterion_main!(benches);
